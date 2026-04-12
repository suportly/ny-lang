// src/cdp/mod.rs

pub mod activation;
pub mod identity;
pub mod ingestion;
pub mod processing;
pub mod segmentation;

pub struct CustomerProfile {
    pub id: String,
    pub attributes: serde_json::Value,
}
