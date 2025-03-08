use std::sync::Arc;

use axum::{http::Method, routing::{get, post, put}, Router};
use cqrs::{AddProductToCartCommandHandler, CreateCartCommandHandler, GetCartsQueryHandler};
use events::{RabbitMqInitializationInfo, RabbitMqMessageBroker};
use repositories::{MongoDbCartRepository, MongoDbInitializationInfo, MongoDbOrderRepository};
use routes::{add_product_to_cart, create_cart, get_cart_by_id, index};
use state::AppState;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uow::RepositoryContext;

mod domain;
mod repositories;
mod uow;
mod dtos;
mod cqrs;
mod state;
mod routes;
mod events;

#[tokio::main]
async fn main() {
    let order_db_info = MongoDbInitializationInfo {
        uri: String::from("mongodb://localhost:27017"),
        database: String::from("eshop-order"),
        collection: String::from("orders")
    };

    let cart_db_info = MongoDbInitializationInfo {
        uri: String::from("mongodb://localhost:27017"),
        database: String::from("eshop-order"),
        collection: String::from("carts")
    };

    let order_repository = Arc::new(MongoDbOrderRepository::new(&order_db_info).await);
    let cart_repository = Arc::new(MongoDbCartRepository::new(&cart_db_info).await);
    let message_broker = Arc::new(RabbitMqMessageBroker::new(RabbitMqInitializationInfo::new(String::from("localhost"), 5672, String::from("guest"), String::from("guest"))).await.unwrap());
    let uow = Arc::new(RepositoryContext::new(order_repository, cart_repository, message_broker));
    let create_cart_command_handler = Arc::new(CreateCartCommandHandler::new(uow.clone()));
    let get_carts_query_handle = Arc::new(GetCartsQueryHandler::new(uow.clone()));
    let add_product_to_cart_command_handler = Arc::new(AddProductToCartCommandHandler::new(uow.clone()));

    let state = Arc::new(AppState {
        create_cart_command_handler: create_cart_command_handler,
        get_carts_query_handle: get_carts_query_handle,
        add_product_to_cart_command_handler: add_product_to_cart_command_handler,
    });

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9091").await.unwrap();

    axum::serve(listener, Router::new()
        .route("/", get(index))
        .route("/carts", post(create_cart))
        .route("/carts/{id}", get(get_cart_by_id))
        .route("/carts/addProductToCart", put(add_product_to_cart))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::very_permissive().allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]))
        )).await.unwrap();
}
