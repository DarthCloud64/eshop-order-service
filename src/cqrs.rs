use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{
    domain::Cart,
    dtos::{
        AddProductToCartResponse, CartResponse, CreateCartResponse, EmptyResponse,
        GetCartsResponse, Response,
    },
    events::Event,
    uow::{OrderUnitOfWork, UnitOfWork},
};

// traits
pub trait Command {}
pub trait Query {}

pub trait CommandHandler<C: Command, R: Response> {
    async fn handle(&self, input: &C) -> Result<R, String>;
}

pub trait QueryHandler<Q: Query, R: Response> {
    async fn handle(&self, input: Option<Q>) -> Result<R, String>;
}

#[derive(Serialize, Deserialize)]
pub struct CreateCartCommand {}
impl Command for CreateCartCommand {}

#[derive(Serialize, Deserialize)]
pub struct AddProductToCartCommand {
    pub cart_id: String,
    pub product_id: String,
}
impl Command for AddProductToCartCommand {}

#[derive(Serialize, Deserialize)]
pub struct RemoveProductFromCartCommand {
    pub cart_id: String,
    pub product_id: String,
}
impl Command for RemoveProductFromCartCommand {}

#[derive(Serialize, Deserialize)]
pub struct GetCartsQuery {
    pub id: String,
}
impl Query for GetCartsQuery {}

pub struct CreateCartCommandHandler {
    uow: Arc<OrderUnitOfWork>,
}

impl CreateCartCommandHandler {
    pub fn new(uow: Arc<OrderUnitOfWork>) -> Self {
        CreateCartCommandHandler { uow: uow }
    }
}

impl CommandHandler<CreateCartCommand, CreateCartResponse> for CreateCartCommandHandler {
    async fn handle(&self, _: &CreateCartCommand) -> Result<CreateCartResponse, String> {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("oops")
            .as_millis();

        let domain_cart = Cart {
            id: uuid::Uuid::new_v4().to_string(),
            products: HashMap::new(),
            created_at_utc: since_the_epoch as i64,
            updated_at_utc: since_the_epoch as i64,
            version: 0,
        };

        let cart_repository = self.uow.get_cart_repository().await;
        let session = self.uow.begin_transaction().await;

        match cart_repository
            .create(domain_cart.id.clone(), domain_cart, session)
            .await
        {
            Ok(created_cart) => match self.uow.commit().await {
                Ok(()) => Ok(CreateCartResponse {
                    id: created_cart.id.clone(),
                }),
                Err(e) => {
                    event!(Level::WARN, "Error occurred while adding product: {}", e);
                    Err(e)
                }
            },
            Err(e) => {
                self.uow.rollback().await.unwrap();
                event!(Level::WARN, "Error occurred while adding product: {}", e);
                Err(e)
            }
        }
    }
}

pub struct AddProductToCartCommandHandler {
    uow: Arc<OrderUnitOfWork>,
}

impl AddProductToCartCommandHandler {
    pub fn new(uow: Arc<OrderUnitOfWork>) -> Self {
        AddProductToCartCommandHandler { uow: uow }
    }
}

impl CommandHandler<AddProductToCartCommand, AddProductToCartResponse>
    for AddProductToCartCommandHandler
{
    async fn handle(
        &self,
        input: &AddProductToCartCommand,
    ) -> Result<AddProductToCartResponse, String> {
        if input.cart_id.is_empty() {
            return Err(String::from("Cart ID cannot be null or empty!!!"));
        }

        if input.product_id.is_empty() {
            return Err(String::from("Product ID cannot be null or empty!!!"));
        }

        let cart_repository = self.uow.get_cart_repository().await;

        match cart_repository.read(&input.cart_id).await {
            Ok(mut found_cart) => {
                match found_cart.products.get(&input.product_id) {
                    Some(current_product_quantity) => {
                        found_cart
                            .products
                            .insert(input.product_id.clone(), current_product_quantity + 1);
                    }
                    None => {
                        found_cart.products.insert(input.product_id.clone(), 1);
                    }
                }

                let session = self.uow.begin_transaction().await;

                match cart_repository
                    .update(input.cart_id.clone(), found_cart, session)
                    .await
                {
                    Ok(updated_cart) => {
                        {
                            let events_to_publish = self.uow.get_events_to_publish().await;
                            let mut event_lock = events_to_publish.lock().await;

                            event_lock.push(Event::ProductAddedToCartEvent {
                                product_id: input.product_id.clone(),
                            });
                        }

                        event!(Level::TRACE, "committing");
                        self.uow.commit().await.unwrap();
                        event!(Level::TRACE, "committed");

                        Ok(AddProductToCartResponse {
                            cart_id: updated_cart.id,
                        })
                    }
                    Err(e) => {
                        self.uow.rollback().await.unwrap();

                        event!(
                            Level::WARN,
                            "Failed to update Cart with ID {}: {}",
                            input.cart_id,
                            e
                        );
                        Err(format!(
                            "Failed to update Cart with ID {}: {}",
                            input.cart_id, e
                        ))
                    }
                }
            }
            Err(e) => {
                event!(
                    Level::WARN,
                    "Failed to find Cart with ID {}: {}",
                    input.cart_id,
                    e
                );
                Err(format!(
                    "Failed to find Cart with ID {}: {}",
                    input.cart_id, e
                ))
            }
        }
    }
}

pub struct RemoveProductFromCartCommandHandler {
    uow: Arc<OrderUnitOfWork>,
}

impl RemoveProductFromCartCommandHandler {
    pub fn new(uow: Arc<OrderUnitOfWork>) -> Self {
        RemoveProductFromCartCommandHandler { uow: uow }
    }
}

impl CommandHandler<RemoveProductFromCartCommand, EmptyResponse>
    for RemoveProductFromCartCommandHandler
{
    async fn handle(&self, input: &RemoveProductFromCartCommand) -> Result<EmptyResponse, String> {
        if input.cart_id.is_empty() {
            return Err(String::from("Cart ID cannot be null or empty!!!"));
        }

        if input.product_id.is_empty() {
            return Err(String::from("Product ID cannot be null or empty!!!"));
        }

        let cart_repository = self.uow.get_cart_repository().await;

        match cart_repository.read(&input.cart_id).await {
            Ok(mut found_cart) => {
                match found_cart.products.get(&input.product_id) {
                    Some(current_product_quantity) => {
                        if *current_product_quantity == 1 {
                            found_cart.products.retain(|k, _| *k != input.product_id);
                        } else {
                            found_cart
                                .products
                                .insert(input.product_id.clone(), current_product_quantity - 1);
                        }
                    }
                    None => {
                        return Err(format!("Cart with id {} was not found", input.cart_id));
                    }
                }

                let session = self.uow.begin_transaction().await;

                match cart_repository
                    .update(input.cart_id.clone(), found_cart, session)
                    .await
                {
                    Ok(_) => {
                        {
                            let events_to_publish = self.uow.get_events_to_publish().await;
                            let mut event_lock = events_to_publish.lock().await;

                            event_lock.push(Event::ProductRemovedFromCartEvent {
                                product_id: input.product_id.clone(),
                            });
                        }

                        event!(Level::TRACE, "committing");
                        self.uow.commit().await.unwrap();
                        event!(Level::TRACE, "committed");

                        Ok(EmptyResponse {})
                    }
                    Err(e) => {
                        self.uow.rollback().await.unwrap();

                        event!(
                            Level::WARN,
                            "Failed to update Cart with ID {}: {}",
                            input.cart_id,
                            e
                        );
                        Err(format!(
                            "Failed to update Cart with ID {}: {}",
                            input.cart_id, e
                        ))
                    }
                }
            }
            Err(e) => {
                event!(
                    Level::WARN,
                    "Failed to find Cart with ID {}: {}",
                    input.cart_id,
                    e
                );
                Err(format!(
                    "Failed to find Cart with ID {}: {}",
                    input.cart_id, e
                ))
            }
        }
    }
}

pub struct GetCartsQueryHandler {
    uow: Arc<OrderUnitOfWork>,
}

impl GetCartsQueryHandler {
    pub fn new(uow: Arc<OrderUnitOfWork>) -> Self {
        GetCartsQueryHandler { uow: uow }
    }
}

impl QueryHandler<GetCartsQuery, GetCartsResponse> for GetCartsQueryHandler {
    async fn handle(
        &self,
        input_option: Option<GetCartsQuery>,
    ) -> Result<GetCartsResponse, String> {
        let cart_repository = self.uow.get_cart_repository().await;

        match input_option {
            Some(input) => match cart_repository.read(input.id.as_str()).await {
                Ok(domain_cart) => {
                    let mut carts = Vec::new();

                    carts.push(CartResponse {
                        id: domain_cart.id.clone(),
                        products: domain_cart.products.clone(),
                    });

                    Ok(GetCartsResponse { carts: carts })
                }
                Err(e) => {
                    event!(Level::WARN, "Error occurred while finding cart: {}", e);
                    Err(e)
                }
            },
            None => {
                event!(Level::INFO, "NOT SUPPORTED YET");
                Ok(GetCartsResponse { carts: Vec::new() })
            }
        }
    }
}
