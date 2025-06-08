use std::sync::Arc;

use async_trait::async_trait;
use mongodb::ClientSession;
use tokio::sync::Mutex;
use tracing::{event, Level};

use crate::{
    events::{Event, MessageBroker},
    repositories::{CartRepository, OrderRepository},
};

#[async_trait]
pub trait UnitOfWork {
    async fn get_order_repository(&self) -> Arc<dyn OrderRepository + Send + Sync>;
    async fn get_cart_repository(&self) -> Arc<dyn CartRepository + Send + Sync>;
    async fn get_events_to_publish(&self) -> Arc<Mutex<Vec<Event>>>;
    async fn begin_transaction(&self) -> Arc<Mutex<ClientSession>>;
    async fn commit(&self) -> Result<(), String>;
    async fn rollback(&self) -> Result<(), String>;
}

#[derive(Clone)]
pub struct OrderUnitOfWork {
    order_repository: Arc<dyn OrderRepository + Send + Sync>,
    cart_repository: Arc<dyn CartRepository + Send + Sync>,
    message_broker: Arc<dyn MessageBroker + Send + Sync>,
    events_to_publish: Arc<Mutex<Vec<Event>>>,
    client_session: Arc<Mutex<ClientSession>>,
}

impl OrderUnitOfWork {
    pub fn new(
        order_repository: Arc<dyn OrderRepository + Send + Sync>,
        cart_repository: Arc<dyn CartRepository + Send + Sync>,
        message_broker: Arc<dyn MessageBroker + Send + Sync>,
        client_session: Arc<Mutex<ClientSession>>,
    ) -> OrderUnitOfWork {
        OrderUnitOfWork {
            order_repository: order_repository,
            cart_repository: cart_repository,
            message_broker: message_broker,
            events_to_publish: Arc::new(Mutex::new(Vec::new())),
            client_session: client_session,
        }
    }
}

#[async_trait]
impl UnitOfWork for OrderUnitOfWork {
    async fn get_order_repository(&self) -> Arc<dyn OrderRepository + Send + Sync> {
        self.order_repository.clone()
    }

    async fn get_cart_repository(&self) -> Arc<dyn CartRepository + Send + Sync> {
        self.cart_repository.clone()
    }

    async fn get_events_to_publish(&self) -> Arc<Mutex<Vec<Event>>> {
        self.events_to_publish.clone()
    }

    async fn begin_transaction(&self) -> Arc<Mutex<ClientSession>> {
        self.client_session
            .lock()
            .await
            .start_transaction()
            .await
            .unwrap();

        self.client_session.clone()
    }
    async fn commit(&self) -> Result<(), String> {
        event!(Level::TRACE, "Committing changes");

        self.client_session
            .lock()
            .await
            .commit_transaction()
            .await
            .unwrap();

        let mut lock = self.events_to_publish.lock().await;
        let mut event_results = Vec::new();
        for e in lock.iter() {
            event!(Level::TRACE, "publishing event");
            event_results.push(self.message_broker.publish_message(e).await);
        }

        let mut single_event_failed = false;
        for result in event_results {
            let _ = match result {
                Ok(()) => (),
                Err(e) => {
                    single_event_failed = true;
                    event!(Level::WARN, "event error found! {}", e);
                }
            };
        }

        lock.clear();

        if single_event_failed {
            return Err(String::from("Failed to commit changes."));
        }

        Ok(())
    }

    async fn rollback(&self) -> Result<(), String> {
        self.client_session
            .lock()
            .await
            .abort_transaction()
            .await
            .unwrap();

        Ok(())
    }
}
