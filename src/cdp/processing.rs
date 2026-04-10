//! src/cdp/processing.rs
//!
//! Responsible for processing ingested events and unifying them into customer profiles.

use crate::cdp::models::{CustomerProfile, Event};
use std::collections::HashMap;

/// The ProfileUnificationEngine merges events into customer profiles.
///
/// It maintains the state of all customer profiles in memory. In a large-scale
/// system, this would be backed by a high-performance database or key-value store.
pub struct ProfileUnificationEngine {
    profiles: HashMap<String, CustomerProfile>,
}

impl ProfileUnificationEngine {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Processes a single event and updates the corresponding customer profile.
    pub fn process_event(&mut self, event: &Event) {
        let profile = self
            .profiles
            .entry(event.customer_id.clone())
            .or_insert_with(|| CustomerProfile::new(event.customer_id.clone()));

        // Merge event properties into the profile's attributes.
        // A more sophisticated strategy could be used here, like handling
        // last-touch, first-touch, or multi-touch attribution.
        for (key, value) in &event.properties {
            profile.attributes.insert(key.clone(), value.clone());
        }

        // Record the event in the profile's history.
        profile.event_history.push(event.event_id.clone());

        // Update a special 'last_seen' attribute.
        profile.attributes.insert(
            "last_seen_ts".to_string(),
            crate::cdp::models::DataValue::Int(event.timestamp as i64),
        );
    }

    /// Retrieves a customer profile by its ID.
    pub fn get_profile(&self, customer_id: &str) -> Option<&CustomerProfile> {
        self.profiles.get(customer_id)
    }

    /// Returns all customer profiles.
    pub fn get_all_profiles(&self) -> Vec<&CustomerProfile> {
        self.profiles.values().collect()
    }
}

impl Default for ProfileUnificationEngine {
    fn default() -> Self {
        Self::new()
    }
}
