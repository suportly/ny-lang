// src/cdp/mod.rs

pub mod activation;
pub mod identity_resolution;
pub mod ingestion;
pub mod processing;
pub mod segmentation;

#[derive(Debug, Clone, PartialEq)]
pub struct CustomerProfile {
    pub id: String,
    pub attributes: serde_json::Value,
}
