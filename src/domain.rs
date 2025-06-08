use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub products: Vec<String>,
    pub payment_id: String,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub id: String,
    pub products: HashMap<String, i32>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: u32,
}
