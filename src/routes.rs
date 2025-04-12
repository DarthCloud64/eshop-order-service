use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{cqrs::{AddProductToCartCommand, CommandHandler, CreateCartCommand, GetCartsQuery, QueryHandler, RemoveProductFromCartCommand}, dtos::ApiError, state::AppState};

pub async fn index() -> &'static str {
    "Hello, World!"
}

pub async fn get_cart_by_id(Path(id): Path<String>, State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>){
    let input = GetCartsQuery {
        id: id.to_string()
    };

    match state.get_carts_query_handle.handle(Some(input)).await {
        Ok(response)=> (StatusCode::OK, Json(json!(response))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(ApiError{error: e})))
    }
}

pub async fn create_cart(state: State<Arc<AppState>>, Json(create_cart_command): Json<CreateCartCommand>) -> (StatusCode, Json<Value>) {
    match state.create_cart_command_handler.handle(&create_cart_command).await {
        Ok(response) => (StatusCode::CREATED, Json(json!(response))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(ApiError{error: e})))
    }
}

pub async fn add_product_to_cart(state: State<Arc<AppState>>, Json(add_product_to_cart_command): Json<AddProductToCartCommand>) -> (StatusCode, Json<Value>) {
    match state.add_product_to_cart_command_handler.handle(&add_product_to_cart_command).await {
        Ok(response) => (StatusCode::OK, Json(json!(response))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(ApiError{error: e})))
    }
}

pub async fn remove_product_from_cart(state: State<Arc<AppState>>, Json(remove_product_from_cart_command): Json<RemoveProductFromCartCommand>) -> (StatusCode, Json<Value>) {
    match state.remove_product_from_cart_command_handler.handle(&remove_product_from_cart_command).await {
        Ok(response) => (StatusCode::NO_CONTENT, Json(json!(response))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(ApiError{error: e})))
    }
}