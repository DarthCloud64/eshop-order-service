use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{domain::{Cart, Order}, repositories::{CartRepository, OrderRepository}};

pub struct RepositoryContext<T1: OrderRepository, T2: CartRepository>{
    pub order_repository: Arc<T1>,
    pub cart_repository: Arc<T2>,
    new_orders: Arc<Mutex<HashMap<String, Order>>>,
    new_carts: Arc<Mutex<HashMap<String, Cart>>>,
}

impl<T1: OrderRepository, T2: CartRepository> RepositoryContext<T1, T2>{
    pub fn new(order_repository: Arc<T1>, cart_repository: Arc<T2>) -> RepositoryContext<T1, T2>{
        RepositoryContext {
            order_repository: order_repository,
            cart_repository: cart_repository,
            new_orders: Arc::new(Mutex::new(HashMap::new())),
            new_carts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_order(&self, id: String, order: Order) -> Result<Order, String>{
        let mut lock = self.new_orders.lock().await;
        lock.insert(id.clone(), order.clone());

        self.order_repository.create(id, order).await
    }

    pub async fn add_cart(&self, id: String, cart: Cart) -> Result<Cart, String>{
        let mut lock = self.new_carts.lock().await;
        lock.insert(id.clone(), cart.clone());
        
        self.cart_repository.create(id, cart).await
    }

    pub async fn commit(&self) -> Result<(), String> {
        let mut order_lock = self.new_orders.lock().await;
        let mut cart_lock = self.new_carts.lock().await;
        order_lock.clear();
        cart_lock.clear();

        Ok(())
    }

    pub async fn rollback(&self) -> Result<(), String> {
        let mut order_lock = self.new_orders.lock().await;
        let mut cart_lock = self.new_carts.lock().await;
        order_lock.clear();
        cart_lock.clear();
        
        Ok(())
    }
}