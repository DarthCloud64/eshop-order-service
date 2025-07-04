use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures_util::TryStreamExt;
use mongodb::{bson::doc, Client, ClientSession, Collection};
use tokio::sync::Mutex;

use crate::domain::{Cart, Order};

#[derive(Debug)]
pub struct MongoDbInitializationInfo {
    pub uri: String,
    pub database: String,
    pub collection: String,
}

#[async_trait]
pub trait OrderRepository {
    async fn create(
        &self,
        id: String,
        order: Order,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String>;
    async fn read<'a>(&self, id: &'a str) -> Result<Order, String>;
    async fn read_all(&self) -> Result<Vec<Order>, String>;
    async fn update(
        &self,
        id: String,
        order: Order,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String>;
    async fn delete(&self, id: &str, session: Arc<Mutex<ClientSession>>);
}

#[async_trait]
pub trait CartRepository {
    async fn create(
        &self,
        id: String,
        cart: Cart,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String>;
    async fn read<'a>(&self, id: &'a str) -> Result<Cart, String>;
    async fn read_all(&self) -> Result<Vec<Cart>, String>;
    async fn update(
        &self,
        id: String,
        cart: Cart,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String>;
    async fn delete(&self, id: &str, session: Arc<Mutex<ClientSession>>);
}

#[derive(Clone)]
pub struct InMemoryOrderRepository {
    orders: Arc<Mutex<HashMap<String, Order>>>,
}

#[derive(Clone)]
pub struct InMemoryCartRepository {
    carts: Arc<Mutex<HashMap<String, Cart>>>,
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        InMemoryOrderRepository {
            orders: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl InMemoryCartRepository {
    pub fn new() -> Self {
        InMemoryCartRepository {
            carts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn create(
        &self,
        id: String,
        order: Order,
        _: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String> {
        let mut lock = self.orders.lock().await;
        lock.insert(id.clone(), order.clone());
        match lock.get(id.as_str()) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Order with id {} did not exist", id)),
        }
    }

    async fn read<'a>(&self, id: &'a str) -> Result<Order, String> {
        let lock = self.orders.lock().await;
        match lock.get(id) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Order with id {} did not exist", id)),
        }
    }

    async fn read_all(&self) -> Result<Vec<Order>, String> {
        let mut orders_to_return = Vec::new();
        let lock = self.orders.lock().await;

        for (_, value) in lock.iter() {
            orders_to_return.push(value.clone());
        }

        Ok(orders_to_return)
    }

    async fn update(
        &self,
        id: String,
        order: Order,
        _: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String> {
        let mut lock = self.orders.lock().await;
        lock.insert(id.clone(), order.clone());
        match lock.get(id.as_str()) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Order with id {} did not exist", id)),
        }
    }

    async fn delete(&self, id: &str, _: Arc<Mutex<ClientSession>>) {
        let mut lock = self.orders.lock().await;
        lock.remove_entry(id);
    }
}

#[async_trait]
impl CartRepository for InMemoryCartRepository {
    async fn create(
        &self,
        id: String,
        cart: Cart,
        _: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String> {
        let mut lock = self.carts.lock().await;
        lock.insert(id.clone(), cart.clone());
        match lock.get(id.as_str()) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Cart with id {} did not exist", id)),
        }
    }

    async fn read<'a>(&self, id: &'a str) -> Result<Cart, String> {
        let lock = self.carts.lock().await;
        match lock.get(id) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Cart with id {} did not exist", id)),
        }
    }

    async fn read_all(&self) -> Result<Vec<Cart>, String> {
        let mut orders_to_return = Vec::new();
        let lock = self.carts.lock().await;

        for (_, value) in lock.iter() {
            orders_to_return.push(value.clone());
        }

        Ok(orders_to_return)
    }

    async fn update(
        &self,
        id: String,
        cart: Cart,
        _: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String> {
        let mut lock = self.carts.lock().await;
        lock.insert(id.clone(), cart.clone());
        match lock.get(id.as_str()) {
            Some(x) => Ok(x.clone()),
            None => Err(format!("Cart with id {} did not exist", id)),
        }
    }

    async fn delete(&self, id: &str, _: Arc<Mutex<ClientSession>>) {
        let mut lock = self.carts.lock().await;
        lock.remove_entry(id);
    }
}

#[derive(Clone)]
pub struct MongoDbOrderRepository {
    order_collection: Collection<Order>,
}

#[derive(Clone)]
pub struct MongoDbCartRepository {
    cart_collection: Collection<Cart>,
}

impl MongoDbOrderRepository {
    pub async fn new(info: &MongoDbInitializationInfo, client: &Client) -> Self {
        let database = client.database(&info.database);

        MongoDbOrderRepository {
            order_collection: database.collection(&info.collection),
        }
    }
}

impl MongoDbCartRepository {
    pub async fn new(info: &MongoDbInitializationInfo, client: &Client) -> Self {
        let database = client.database(&info.database);

        MongoDbCartRepository {
            cart_collection: database.collection(&info.collection),
        }
    }
}

#[async_trait]
impl OrderRepository for MongoDbOrderRepository {
    async fn create(
        &self,
        id: String,
        order: Order,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String> {
        let mut guard = session.lock().await;

        match self
            .order_collection
            .insert_one(order)
            .session(&mut *guard)
            .await
        {
            Ok(_) => match self
                .order_collection
                .find_one(doc! {"id": &id})
                .session(&mut *guard)
                .await
            {
                Ok(find_one_order_option) => match find_one_order_option {
                    Some(p) => Ok(p),
                    None => Err(format!("Failed to find Order with id {}", id)),
                },
                Err(e) => Err(format!("Failed to insert Order: {}", e)),
            },
            Err(e) => Err(format!("Failed to insert Order: {}", e)),
        }
    }

    async fn read<'a>(&self, id: &'a str) -> Result<Order, String> {
        match self.order_collection.find_one(doc! {"id": &id}).await {
            Ok(find_one_order_option) => match find_one_order_option {
                Some(p) => Ok(p),
                None => Err(format!("Failed to find Order with id {}", id)),
            },
            Err(e) => Err(format!("Failed to insert Order: {}", e)),
        }
    }

    async fn read_all(&self) -> Result<Vec<Order>, String> {
        let mut orders_to_return = Vec::new();

        match self.order_collection.find(doc! {}).await {
            Ok(mut found_orders) => {
                while let Ok(Some(order)) = found_orders.try_next().await {
                    orders_to_return.push(order.clone())
                }

                Ok(orders_to_return)
            }
            Err(_) => Err(format!("Failed to find Orders")),
        }
    }

    async fn update(
        &self,
        id: String,
        order: Order,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Order, String> {
        todo!()
    }

    async fn delete(&self, id: &str, session: Arc<Mutex<ClientSession>>) {
        todo!()
    }
}

#[async_trait]
impl CartRepository for MongoDbCartRepository {
    async fn create(
        &self,
        id: String,
        cart: Cart,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String> {
        let mut guard = session.lock().await;

        match self
            .cart_collection
            .insert_one(cart)
            .session(&mut *guard)
            .await
        {
            Ok(_) => match self
                .cart_collection
                .find_one(doc! {"id": &id})
                .session(&mut *guard)
                .await
            {
                Ok(find_one_cart_option) => match find_one_cart_option {
                    Some(p) => Ok(p),
                    None => Err(format!("Failed to find Cart with id {}", id)),
                },
                Err(e) => Err(format!("Failed to insert Cart: {}", e)),
            },
            Err(e) => Err(format!("Failed to insert Cart: {}", e)),
        }
    }

    async fn read<'a>(&self, id: &'a str) -> Result<Cart, String> {
        match self.cart_collection.find_one(doc! {"id": &id}).await {
            Ok(find_one_cart_option) => match find_one_cart_option {
                Some(p) => Ok(p),
                None => Err(format!("Failed to find Cart with id {}", id)),
            },
            Err(e) => Err(format!("Failed to insert Cart: {}", e)),
        }
    }

    async fn read_all(&self) -> Result<Vec<Cart>, String> {
        let mut carts_to_return = Vec::new();

        match self.cart_collection.find(doc! {}).await {
            Ok(mut found_carts) => {
                while let Ok(Some(cart)) = found_carts.try_next().await {
                    carts_to_return.push(cart.clone())
                }

                Ok(carts_to_return)
            }
            Err(_) => Err(format!("Failed to find Carts")),
        }
    }

    async fn update(
        &self,
        id: String,
        cart: Cart,
        session: Arc<Mutex<ClientSession>>,
    ) -> Result<Cart, String> {
        let mut guard = session.lock().await;

        match self
            .cart_collection
            .replace_one(doc! {"id": &id}, cart)
            .session(&mut *guard)
            .await
        {
            Ok(_) => match self
                .cart_collection
                .find_one(doc! {"id": &id})
                .session(&mut *guard)
                .await
            {
                Ok(find_one_cart_option) => match find_one_cart_option {
                    Some(p) => Ok(p),
                    None => Err(format!("Failed to find Cart with id {}", id)),
                },
                Err(e) => Err(format!("Failed to update Cart: {}", e)),
            },
            Err(e) => Err(format!("Failed to update Cart: {}", e)),
        }
    }

    async fn delete(&self, id: &str, session: Arc<Mutex<ClientSession>>) {
        todo!()
    }
}
