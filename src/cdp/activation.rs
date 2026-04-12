// src/cdp/activation.rs

use super::CustomerProfile;
use std::io::Result;

/// `ActivationTarget` is a trait for systems where customer data is "activated".
/// This could be an email marketing platform, a CRM, an ad network, etc.
pub trait ActivationTarget {
    fn activate(&self, profiles: &[CustomerProfile]) -> Result<()>;
}

/// A mock target that simulates activating profiles by printing them.
/// This could be replaced with a real integration (e.g., an HTTP client for a REST API).
pub struct MockApiTarget;

impl ActivationTarget for MockApiTarget {
    fn activate(&self, profiles: &[CustomerProfile]) -> Result<()> {
        if profiles.is_empty() {
            println!("[Activation] No profiles to activate.");
            return Ok(());
        }

        println!("[Activation] Activating {} profiles:", profiles.len());
        for profile in profiles {
            println!("  - Activating profile ID: {}", profile.id);
            // In a real scenario, you would send profile data to the target system.
            // e.g., serde_json::to_string(profile).and_then(|p| http_client.post(..., p))
        }
        Ok(())
    }
}
