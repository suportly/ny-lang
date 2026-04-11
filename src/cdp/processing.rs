// src/cdp/processing.rs

use super::CustomerProfile;

pub fn process_profiles(profiles: Vec<CustomerProfile>) -> Vec<CustomerProfile> {
    // Example processing: add a new attribute
    profiles
        .into_iter()
        .map(|mut profile| {
            if let Some(obj) = profile.attributes.as_object_mut() {
                obj.insert(
                    "processed_at".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            profile
        })
        .collect()
}
