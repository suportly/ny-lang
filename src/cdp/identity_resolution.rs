// src/cdp/identity_resolution.rs

use crate::cdp::CustomerProfile;
use std::collections::HashMap;

/// Resolves customer identities from multiple profiles.
///
/// This function takes a list of customer profiles and merges them into a single profile
/// based on a common identifier (e.g., email).
///
/// # Arguments
///
/// * `profiles` - A vector of `CustomerProfile` to be resolved.
///
/// # Returns
///
/// * `Option<CustomerProfile>` - The merged customer profile, or `None` if no profiles were provided.
pub fn resolve_identities(profiles: Vec<CustomerProfile>) -> Option<CustomerProfile> {
    if profiles.is_empty() {
        return None;
    }

    let mut merged_attributes = serde_json::Map::new();
    let mut final_id = String::new();

    // Simple identity resolution based on the first profile's ID
    if let Some(first_profile) = profiles.first() {
        final_id = first_profile.id.clone();
    }

    for profile in profiles {
        if let serde_json::Value::Object(attrs) = profile.attributes {
            for (key, value) in attrs {
                merged_attributes.insert(key, value);
            }
        }
    }

    Some(CustomerProfile {
        id: final_id,
        attributes: serde_json::Value::Object(merged_attributes),
    })
}
