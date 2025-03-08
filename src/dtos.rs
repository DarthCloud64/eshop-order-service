use serde::{Deserialize, Serialize};

pub trait Response{}

#[derive(Serialize, Deserialize)]
pub struct CreateCartResponse {
    pub id: String
}
impl Response for CreateCartResponse{}

#[derive(Serialize, Deserialize)]
pub struct CartResponse {
    pub id: String,
    pub products: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetCartsResponse {
    pub carts: Vec<CartResponse>
}
impl Response for GetCartsResponse{}

#[derive(Serialize, Deserialize)]
pub struct AddProductToCartResponse {
    pub cart_id: String
}
impl Response for AddProductToCartResponse{}

#[derive(Serialize, Deserialize)]
pub struct ApiError {
    pub error: String
}
impl Response for ApiError{}