use std::sync::Arc;

use crate::{cqrs::{AddProductToCartCommandHandler, CreateCartCommandHandler, GetCartsQueryHandler, RemoveProductFromCartCommandHandler}, events::RabbitMqMessageBroker, repositories::{MongoDbCartRepository, MongoDbOrderRepository}};

#[derive(Clone)]
pub struct AppState {
    pub create_cart_command_handler: Arc<CreateCartCommandHandler<MongoDbOrderRepository, MongoDbCartRepository, RabbitMqMessageBroker>>,
    pub get_carts_query_handle: Arc<GetCartsQueryHandler<MongoDbOrderRepository, MongoDbCartRepository, RabbitMqMessageBroker>>,
    pub add_product_to_cart_command_handler: Arc<AddProductToCartCommandHandler<MongoDbOrderRepository, MongoDbCartRepository, RabbitMqMessageBroker>>,
    pub remove_product_from_cart_command_handler: Arc<RemoveProductFromCartCommandHandler<MongoDbOrderRepository, MongoDbCartRepository, RabbitMqMessageBroker>>,
    pub auth0_domain: String,
    pub auth0_audience: String,
}