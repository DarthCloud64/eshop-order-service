use std::sync::Arc;

use axum::{
    http::Method,
    middleware::from_fn_with_state,
    routing::{get, post, put},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use cqrs::{
    AddProductToCartCommandHandler, CreateCartCommandHandler, GetCartsQueryHandler,
    RemoveProductFromCartCommandHandler,
};
use dotenv::dotenv;
use events::{RabbitMqInitializationInfo, RabbitMqMessageBroker};
use mongodb::Client;
use repositories::{MongoDbCartRepository, MongoDbInitializationInfo, MongoDbOrderRepository};
use routes::{add_product_to_cart, create_cart, get_cart_by_id, index, remove_product_from_cart};
use state::AppState;
use std::env;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::uow::OrderUnitOfWork;

mod auth;
mod cqrs;
mod domain;
mod dtos;
mod events;
mod repositories;
mod routes;
mod state;
mod uow;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let order_db_info = MongoDbInitializationInfo {
        uri: String::from(env::var("MONGODB_URI").unwrap()),
        database: String::from(env::var("MONGODB_DB").unwrap()),
        collection: String::from(env::var("MONGODB_ORDER_COLLECTION").unwrap()),
    };

    let cart_db_info = MongoDbInitializationInfo {
        uri: String::from(env::var("MONGODB_URI").unwrap()),
        database: String::from(env::var("MONGODB_DB").unwrap()),
        collection: String::from(env::var("MONGODB_CARTS_COLLECTION").unwrap()),
    };

    let client: Client = Client::with_uri_str(&cart_db_info.uri).await.unwrap();

    let order_repository = Arc::new(MongoDbOrderRepository::new(&order_db_info, &client).await);
    let cart_repository = Arc::new(MongoDbCartRepository::new(&cart_db_info, &client).await);

    let message_broker = Arc::new(
        RabbitMqMessageBroker::new(RabbitMqInitializationInfo::new(
            String::from(env::var("RABBITMQ_URI").unwrap()),
            env::var("RABBITMQ_PORT").unwrap().parse().unwrap(),
            String::from(env::var("RABBITMQ_USER").unwrap()),
            String::from(env::var("RABBITMQ_PASS").unwrap()),
        ))
        .await
        .unwrap(),
    );

    let client_session = Arc::new(Mutex::new(client.start_session().await.unwrap()));

    let uow = Arc::new(OrderUnitOfWork::new(
        order_repository,
        cart_repository,
        message_broker,
        client_session,
    ));

    let create_cart_command_handler = Arc::new(CreateCartCommandHandler::new(uow.clone()));
    let get_carts_query_handle = Arc::new(GetCartsQueryHandler::new(uow.clone()));
    let add_product_to_cart_command_handler =
        Arc::new(AddProductToCartCommandHandler::new(uow.clone()));
    let remove_product_from_cart_command_handler =
        Arc::new(RemoveProductFromCartCommandHandler::new(uow.clone()));

    let state = Arc::new(AppState {
        create_cart_command_handler: create_cart_command_handler,
        get_carts_query_handle: get_carts_query_handle,
        add_product_to_cart_command_handler: add_product_to_cart_command_handler,
        remove_product_from_cart_command_handler: remove_product_from_cart_command_handler,
        auth0_domain: String::from(env::var("AUTH0_DOMAIN").unwrap()),
        auth0_audience: String::from(env::var("AUTH0_AUDIENCE").unwrap()),
    });

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_ansi(false)
        .json()
        .with_file(true)
        .with_line_number(true)
        .with_current_span(true)
        .with_writer(std::fs::File::create(String::from(env::var("LOG_PATH").unwrap())).unwrap())
        .init();

    let (prometheus_layer, metrics_handle) = PrometheusMetricLayer::pair();

    let listener =
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env::var("AXUM_PORT").unwrap()))
            .await
            .unwrap();

    axum::serve(
        listener,
        Router::new()
            .route("/", get(index))
            .route("/metrics", get(|| async move { metrics_handle.render() }))
            .route(
                "/carts",
                post(create_cart).route_layer(from_fn_with_state(
                    state.clone(),
                    auth::authentication_middleware,
                )),
            )
            .route(
                "/carts/{id}",
                get(get_cart_by_id).route_layer(from_fn_with_state(
                    state.clone(),
                    auth::authentication_middleware,
                )),
            )
            .route(
                "/carts/addProductToCart",
                put(add_product_to_cart).route_layer(from_fn_with_state(
                    state.clone(),
                    auth::authentication_middleware,
                )),
            )
            .route(
                "/carts/removeProductFromCart",
                put(remove_product_from_cart).route_layer(from_fn_with_state(
                    state.clone(),
                    auth::authentication_middleware,
                )),
            )
            .with_state(state)
            .layer(prometheus_layer)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::very_permissive().allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::DELETE,
                    ])),
            ),
    )
    .await
    .unwrap();
}
