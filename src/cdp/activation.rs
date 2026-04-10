//! src/cdp/activation.rs
//!
//! Activates segmented data by sending it to external systems for marketing,
//! analytics, or AI/ML model serving.

use crate::cdp::models::CustomerProfile;

/// A trait for activation targets.
///
/// This allows for sending segmented customer data to various destinations,
/// such as an email marketing platform, a push notification service, or an
/// ML model inference endpoint.
pub trait ActivationTarget {
    fn activate(&self, profiles: Vec<&CustomerProfile>) -> Result<ActivationResult, String>;
}

/// The result of an activation task.
#[derive(Debug, PartialEq)]
pub struct ActivationResult {
    pub target_name: String,
    pub successful_activations: u32,
    pub failed_activations: u32,
}

/// A mock implementation of an ActivationTarget for demonstration and testing.
pub struct MockTarget {
    name: String,
    // A flag to simulate failures for testing purposes.
    should_fail: bool,
}

impl MockTarget {
    pub fn new(name: &str, should_fail: bool) -> Self {
        Self {
            name: name.to_string(),
            should_fail,
        }
    }
}

impl ActivationTarget for MockTarget {
    fn activate(&self, profiles: Vec<&CustomerProfile>) -> Result<ActivationResult, String> {
        if self.should_fail {
            Err(format!("MockTarget '{}' failed to activate.", self.name))
        } else {
            println!(
                "Activating {} profiles for target '{}'...",
                profiles.len(),
                self.name
            );
            // In a real implementation, this would involve API calls or other
            // interactions with the external system.
            for profile in &profiles {
                println!("  - Activating profile: {}", profile.customer_id);
            }

            Ok(ActivationResult {
                target_name: self.name.clone(),
                successful_activations: profiles.len() as u32,
                failed_activations: 0,
            })
        }
    }
}
