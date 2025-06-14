use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicPublishArguments, Channel, ExchangeDeclareArguments, ExchangeType, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    BasicProperties, DELIVERY_MODE_PERSISTENT,
};
use async_trait::async_trait;
use serde::Serialize;

pub static PRODUCT_ADDED_TO_CART_QUEUE_NAME: &str = "product.added.to.cart";
pub static PRODUCT_REMOVED_FROM_CART_QUEUE_NAME: &str = "product.removed.from.cart";

pub struct RabbitMqInitializationInfo {
    uri: String,
    port: u16,
    username: String,
    password: String,
}

impl RabbitMqInitializationInfo {
    pub fn new(
        uri: String,
        port: u16,
        username: String,
        password: String,
    ) -> RabbitMqInitializationInfo {
        RabbitMqInitializationInfo {
            uri: uri,
            port: port,
            username: username,
            password: password,
        }
    }
}

#[derive(Serialize)]
pub enum Event {
    ProductAddedToCartEvent { product_id: String },
    ProductRemovedFromCartEvent { product_id: String },
}

#[async_trait]
pub trait MessageBroker {
    async fn publish_message(&self, event: &Event) -> Result<(), String>;
}

pub struct RabbitMqMessageBroker {
    connection: Connection,
}

impl RabbitMqMessageBroker {
    pub async fn new(
        init_info: RabbitMqInitializationInfo,
    ) -> Result<RabbitMqMessageBroker, String> {
        match Connection::open(&OpenConnectionArguments::new(
            &init_info.uri,
            init_info.port,
            &init_info.username,
            &init_info.password,
        ))
        .await
        {
            Ok(connection) => {
                match connection
                    .register_callback(DefaultConnectionCallback)
                    .await
                {
                    Ok(()) => Ok(RabbitMqMessageBroker {
                        connection: connection,
                    }),
                    Err(e) => Err(format!("Failed to register connection callback: {}", e)),
                }
            }
            Err(e) => Err(format!("Failed to open RabbitMQ connection: {}", e)),
        }
    }

    pub async fn get_channel(&self, destination: &str) -> Result<Channel, String> {
        match self.connection.open_channel(None).await {
            Ok(channel) => {
                channel
                    .register_callback(DefaultChannelCallback)
                    .await
                    .unwrap();
                channel
                    .exchange_declare(ExchangeDeclareArguments::new(
                        destination,
                        &ExchangeType::Fanout.to_string(),
                    ))
                    .await
                    .unwrap();
                channel
                    .queue_declare(QueueDeclareArguments::durable_client_named(destination))
                    .await
                    .unwrap();
                channel
                    .queue_bind(QueueBindArguments::new(destination, destination, ""))
                    .await
                    .unwrap();

                Ok(channel)
            }
            Err(e) => Err(format!("Failed to get channel: {}", e)),
        }
    }
}

#[async_trait]
impl MessageBroker for RabbitMqMessageBroker {
    async fn publish_message(&self, event: &Event) -> Result<(), String> {
        let mut destination_name = String::new();
        match event {
            Event::ProductAddedToCartEvent { .. } => {
                destination_name = String::from(PRODUCT_ADDED_TO_CART_QUEUE_NAME);
            }
            Event::ProductRemovedFromCartEvent { .. } => {
                destination_name = String::from(PRODUCT_REMOVED_FROM_CART_QUEUE_NAME);
            }
        }

        match self.get_channel(&destination_name).await {
            Ok(channel) => {
                let mut delivery_properties = BasicProperties::default();
                delivery_properties.with_delivery_mode(DELIVERY_MODE_PERSISTENT);

                match serde_json::to_string(&event) {
                    Ok(x) => {
                        match channel
                            .basic_publish(
                                delivery_properties,
                                x.into_bytes(),
                                BasicPublishArguments::new(&destination_name, ""),
                            )
                            .await
                        {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("Failed to publish event to broker: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to serialize event: {}", e)),
                }
            }
            Err(e) => Err(format!("Failed to publish event to broker: {}", e)),
        }
    }
}
