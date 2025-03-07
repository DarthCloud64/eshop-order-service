use std::sync::Arc;

use crate::{cqrs::{CreateCartCommandHandler, GetCartsQueryHandler}, repositories::{MongoDbCartRepository, MongoDbOrderRepository}};

#[derive(Clone)]
pub struct AppState {
    pub create_cart_command_handler: Arc<CreateCartCommandHandler<MongoDbOrderRepository, MongoDbCartRepository>>,
    pub get_carts_query_handle: Arc<GetCartsQueryHandler<MongoDbOrderRepository, MongoDbCartRepository>>
}