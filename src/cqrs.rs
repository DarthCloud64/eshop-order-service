use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{domain::Cart, dtos::{CartResponse, CreateCartResponse, GetCartsResponse, Response}, repositories::{CartRepository, OrderRepository}, uow::RepositoryContext};

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
    pub products: Vec<String>,
}
impl Command for CreateCartCommand{}

#[derive(Serialize, Deserialize)]
pub struct GetCartsQuery {
    pub id: String
}
impl Query for GetCartsQuery{}

pub struct CreateCartCommandHandler<T1: OrderRepository, T2: CartRepository>{
    uow: Arc<RepositoryContext<T1, T2>>
}

impl<T1: OrderRepository, T2: CartRepository> CreateCartCommandHandler<T1, T2>{
    pub fn new(uow: Arc<RepositoryContext<T1, T2>>) -> Self{
        CreateCartCommandHandler {
            uow: uow
        }
    }
}

impl<T1: OrderRepository, T2: CartRepository> CommandHandler<CreateCartCommand, CreateCartResponse> for CreateCartCommandHandler<T1, T2>{
    async fn handle(&self, input: &CreateCartCommand) -> Result<CreateCartResponse, String> {
        if input.products.is_empty() {
            return Err(String::from("Products cannot be empty!!!"));
        }

        let domain_cart = Cart {
            id: uuid::Uuid::new_v4().to_string(),
            products: input.products.clone()
        };

        match self.uow.add_cart(domain_cart.id.clone(), domain_cart).await {
            Ok(created_cart) => {
                match self.uow.commit().await {
                    Ok(()) => Ok(CreateCartResponse {
                        id: created_cart.id.clone()
                    }),
                    Err(e) => {
                        println!("Error occurred while adding product: {}", e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                println!("Error occurred while adding product: {}", e);
                Err(e)
            }
        }
    }
}

pub struct GetCartsQueryHandler<T1: OrderRepository, T2: CartRepository> {
    uow: Arc<RepositoryContext<T1, T2>>
}

impl<T1: OrderRepository, T2: CartRepository> GetCartsQueryHandler<T1, T2> {
    pub fn new(uow: Arc<RepositoryContext<T1, T2>>) -> Self {
        GetCartsQueryHandler {
            uow: uow
        }
    }
}

impl<T1: OrderRepository, T2: CartRepository> QueryHandler<GetCartsQuery, GetCartsResponse> for GetCartsQueryHandler<T1, T2> {
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
                        println!("Error occurred while finding cart: {}", e);
                        Err(e)
                    }
                }
            },
            None => {
                println!("NOT SUPPORTED YET");
                Ok(GetCartsResponse{carts: Vec::new()})
            }
        }
    }
}