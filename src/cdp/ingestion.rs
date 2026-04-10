//! src/cdp/ingestion.rs
//!
//! Handles real-time data ingestion from various sources.

use crate::cdp::models::Event;
use std::sync::mpsc::{channel, Receiver, Sender};

/// A trait for data sources that can be ingested into the CDP.
///
/// This allows for a pluggable architecture for different data sources
/// like webhooks, Kafka streams, file uploads, etc.
pub trait IngestionSource {
    fn ingest(&mut self) -> Result<Event, String>;
}

/// An IngestionStream manages the flow of events from a source
/// into a channel for processing.
pub struct IngestionStream {
    // In a real system, this would be a more robust, distributed message queue.
    // std::sync::mpsc is used here for simplicity.
    sender: Sender<Event>,
}

impl IngestionStream {
    /// Creates a new ingestion stream and a receiver to process events.
    pub fn new() -> (Self, Receiver<Event>) {
        let (sender, receiver) = channel();
        (Self { sender }, receiver)
    }

    /// Pushes a new event into the stream.
    pub fn push(&self, event: Event) -> Result<(), String> {
        self.sender
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))
    }
}

/// A mock implementation of an IngestionSource for demonstration and testing.
pub struct MockSource {
    events: Vec<Event>,
}

impl MockSource {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events }
    }
}

impl IngestionSource for MockSource {
    fn ingest(&mut self) -> Result<Event, String> {
        self.events
            .pop()
            .ok_or_else(|| "No more events in mock source".to_string())
    }
}
