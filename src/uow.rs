use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use tracing::{event, Level};

use crate::{domain::{Cart, Order}, events::{Event, MessageBroker}, repositories::{CartRepository, OrderRepository}};

pub struct RepositoryContext<T1: OrderRepository, T2: CartRepository, T3: MessageBroker>{
    pub order_repository: Arc<T1>,
    pub cart_repository: Arc<T2>,
    pub events_to_publish: Arc<Mutex<Vec<Event>>>,
    message_broker: Arc<T3>,
    new_orders: Arc<Mutex<HashMap<String, Order>>>,
    new_carts: Arc<Mutex<HashMap<String, Cart>>>,
}

impl<T1: OrderRepository, T2: CartRepository, T3: MessageBroker> RepositoryContext<T1, T2, T3>{
    pub fn new(order_repository: Arc<T1>, cart_repository: Arc<T2>, message_broker: Arc<T3>) -> RepositoryContext<T1, T2, T3>{
        RepositoryContext {
            order_repository: order_repository,
            cart_repository: cart_repository,
            message_broker: message_broker,
            new_orders: Arc::new(Mutex::new(HashMap::new())),
            new_carts: Arc::new(Mutex::new(HashMap::new())),
            events_to_publish: Arc::new(Mutex::new(Vec::new())),
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
        event!(Level::TRACE, "cleared!!");

        let mut event_lock = self.events_to_publish.lock().await;
        let mut event_results = Vec::new();
        event!(Level::TRACE, "gonna loop");
        for e in event_lock.iter(){
            event!(Level::TRACE, "publishing event");
            event_results.push(self.message_broker.publish_message(e, "product.added.to.cart").await);
        }

        let mut single_event_failed = false;
        for result in event_results{
            let _ = match result {
                Ok(()) => (),
                Err(e) => {
                    single_event_failed = true;
                    event!(Level::WARN, "event error found! {}", e);
                }
            };
        }

        event_lock.clear();
        
        if single_event_failed {
            return Err(String::from("Failed to commit changes."))
        }

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