// src/cdp/identity.rs

use super::CustomerProfile;
use std::collections::{HashMap, HashSet};

/// Merges customer profiles based on a unique identifier (e.g., email).
///
/// This is a simplified identity resolution strategy. A real-world implementation
/// would involve more sophisticated fuzzy matching and conflict resolution logic.
pub fn resolve_identities(profiles: Vec<CustomerProfile>, on: &str) -> Vec<CustomerProfile> {
    let mut profile_map: HashMap<String, CustomerProfile> = HashMap::new();
    let mut processed_ids: HashSet<String> = HashSet::new();

    for profile in profiles {
        if processed_ids.contains(&profile.id) {
            continue;
        }

        if let Some(key) = profile.attributes.get(on).and_then(|v| v.as_str()) {
            let key = key.to_string();
            if let Some(existing_profile) = profile_map.get_mut(&key) {
                // Merge attributes. In case of conflict, new profile's data wins.
                if let (Some(existing_obj), Some(new_obj)) =
                    (existing_profile.attributes.as_object_mut(), profile.attributes.as_object())
                {
                    for (k, v) in new_obj {
                        existing_obj.insert(k.clone(), v.clone());
                    }
                }
                processed_ids.insert(profile.id);
            } else {
                profile_map.insert(key, profile);
            }
        } else {
            // Profile without the key is treated as unique for now
            profile_map.insert(profile.id.clone(), profile);
        }
    }

    profile_map.into_values().collect()
}
