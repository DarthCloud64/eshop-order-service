use std::sync::Arc;

use axum::{http::Method, routing::{get, post}, Router};
use cqrs::{CreateCartCommandHandler, GetCartsQueryHandler};
use repositories::{MongoDbCartRepository, MongoDbInitializationInfo, MongoDbOrderRepository};
use routes::{create_cart, get_cart_by_id, index};
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
    let uow = Arc::new(RepositoryContext::new(order_repository, cart_repository));
    let create_cart_command_handler = Arc::new(CreateCartCommandHandler::new(uow.clone()));
    let get_carts_query_handle = Arc::new(GetCartsQueryHandler::new(uow.clone()));

    let state = Arc::new(AppState {
        create_cart_command_handler: create_cart_command_handler,
        get_carts_query_handle
    });

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9091").await.unwrap();

    axum::serve(listener, Router::new()
        .route("/", get(index))
        .route("/carts", post(create_cart))
        .route("/carts/{id}", get(get_cart_by_id))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::very_permissive().allow_methods([Method::GET, Method::POST]))
        )).await.unwrap();
}
