use std::sync::Arc;

use crate::cqrs::{
    AddProductToCartCommandHandler, CreateCartCommandHandler, GetCartsQueryHandler,
    RemoveProductFromCartCommandHandler,
};

#[derive(Clone)]
pub struct AppState {
    pub create_cart_command_handler: Arc<CreateCartCommandHandler>,
    pub get_carts_query_handle: Arc<GetCartsQueryHandler>,
    pub add_product_to_cart_command_handler: Arc<AddProductToCartCommandHandler>,
    pub remove_product_from_cart_command_handler: Arc<RemoveProductFromCartCommandHandler>,
    pub auth0_domain: String,
    pub auth0_audience: String,
}
