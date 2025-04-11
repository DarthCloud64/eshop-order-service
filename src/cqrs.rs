use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{domain::Cart, dtos::{AddProductToCartResponse, CartResponse, CreateCartResponse, GetCartsResponse, Response}, events::{Event, MessageBroker}, repositories::{CartRepository, OrderRepository}, uow::RepositoryContext};

// traits
pub trait Command{}
pub trait Query{}

pub trait CommandHandler<C: Command, R: Response>{
    async fn handle(&self, input: &C) -> Result<R, String>;
}

pub trait QueryHandler<Q: Query, R: Response>{
    async fn handle(&self, input: Option<Q>) -> Result<R, String>;
}

#[derive(Serialize, Deserialize)]
pub struct CreateCartCommand{
}
impl Command for CreateCartCommand{}

#[derive(Serialize, Deserialize)]
pub struct AddProductToCartCommand {
    pub cart_id: String,
    pub product_id: String,
}
impl Command for AddProductToCartCommand{}

#[derive(Serialize, Deserialize)]
pub struct GetCartsQuery {
    pub id: String
}
impl Query for GetCartsQuery{}

pub struct CreateCartCommandHandler<T1: OrderRepository, T2: CartRepository, T3: MessageBroker>{
    uow: Arc<RepositoryContext<T1, T2, T3>>
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> CreateCartCommandHandler<T1, T2, T3>{
    pub fn new(uow: Arc<RepositoryContext<T1, T2, T3>>) -> Self{
        CreateCartCommandHandler {
            uow: uow
        }
    }
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> CommandHandler<CreateCartCommand, CreateCartResponse> for CreateCartCommandHandler<T1, T2, T3>{
    async fn handle(&self, input: &CreateCartCommand) -> Result<CreateCartResponse, String> {
        let domain_cart = Cart {
            id: uuid::Uuid::new_v4().to_string(),
            products: HashMap::new()
        };

        match self.uow.add_cart(domain_cart.id.clone(), domain_cart).await {
            Ok(created_cart) => {
                match self.uow.commit().await {
                    Ok(()) => Ok(CreateCartResponse {
                        id: created_cart.id.clone()
                    }),
                    Err(e) => {
                        event!(Level::WARN, "Error occurred while adding product: {}", e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                event!(Level::WARN, "Error occurred while adding product: {}", e);
                Err(e)
            }
        }
    }
}

pub struct AddProductToCartCommandHandler<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> {
    uow: Arc<RepositoryContext<T1, T2, T3>>
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> AddProductToCartCommandHandler<T1, T2, T3>{
    pub fn new(uow: Arc<RepositoryContext<T1, T2, T3>>) -> Self {
        AddProductToCartCommandHandler{
            uow: uow
        }
    }
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> CommandHandler<AddProductToCartCommand, AddProductToCartResponse> for AddProductToCartCommandHandler<T1, T2, T3>{
    async fn handle(&self, input: &AddProductToCartCommand) -> Result<AddProductToCartResponse, String> {
        if input.cart_id.is_empty() {
            return Err(String::from("Cart ID cannot be null or empty!!!"));
        }

        if input.product_id.is_empty() {
            return Err(String::from("Product ID cannot be null or empty!!!"));
        }

        match self.uow.cart_repository.read(&input.cart_id).await {
            Ok(mut found_cart) => {
                match found_cart.products.get(&input.product_id) {
                    Some(current_product_quantity) => {
                        found_cart.products.insert(input.product_id.clone(), current_product_quantity + 1);
                    },
                    None => {
                        found_cart.products.insert(input.product_id.clone(), 1);
                    }
                }

                match self.uow.cart_repository.update(input.cart_id.clone(), found_cart).await{
                    Ok(updated_cart) => {
                        {
                            let mut event_lock = self.uow.events_to_publish.lock().await;

                            event_lock.push(Event::ProductAddedToCartEvent{
                                product_id: input.product_id.clone()
                            });
                        }

                        event!(Level::TRACE, "committing");
                        self.uow.commit().await.unwrap();
                        event!(Level::TRACE, "committed");

                        Ok(AddProductToCartResponse {
                            cart_id: updated_cart.id
                        })
                    },
                    Err(e) => {
                        Err(format!("Failed to update Cart with ID {}: {}", input.cart_id, e))
                    }
                }
            },
            Err(e) => {
                Err(format!("Failed to find Cart with ID {}: {}", input.cart_id, e))
            }
        }
    }
}

pub struct GetCartsQueryHandler<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> {
    uow: Arc<RepositoryContext<T1, T2, T3>>
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> GetCartsQueryHandler<T1, T2, T3> {
    pub fn new(uow: Arc<RepositoryContext<T1, T2, T3>>) -> Self {
        GetCartsQueryHandler {
            uow: uow
        }
    }
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> QueryHandler<GetCartsQuery, GetCartsResponse> for GetCartsQueryHandler<T1, T2, T3> {
    async fn handle(&self, input_option: Option<GetCartsQuery>) -> Result<GetCartsResponse, String> {
        match input_option {
            Some(input) => {
                match self.uow.cart_repository.read(input.id.as_str()).await {
                    Ok(domain_cart) => {
                        let mut carts = Vec::new();

                        carts.push(CartResponse {
                            id: domain_cart.id.clone(),
                            products: domain_cart.products.clone()
                        });

                        Ok(GetCartsResponse {
                            carts: carts
                        })
                    },
                    Err(e) => {
                        event!(Level::WARN, "Error occurred while finding cart: {}", e);
                        Err(e)
                    }
                }
            },
            None => {
                event!(Level::INFO, "NOT SUPPORTED YET");
                Ok(GetCartsResponse{carts: Vec::new()})
            }
        }
    }
}