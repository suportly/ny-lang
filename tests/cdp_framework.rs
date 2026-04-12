// tests/cdp_framework.rs

use ny::cdp::{
    activation::{activate_profile, ActivationTarget},
    identity_resolution::resolve_identities,
    ingestion::{CsvSource, IngestionSource},
    processing::process_profiles,
    segmentation::{segment_profiles, Segment},
    CustomerProfile,
};
use serde_json::json;

#[test]
fn test_csv_ingestion() {
    // This test will fail if dummy.csv is not present.
    // For the purpose of this example, we'll create it.
    std::fs::write("dummy.csv", "id,name\nuser123,Alice").unwrap();
    let source = CsvSource { path: "dummy.csv" };
    let profiles = source.ingest().unwrap();
    assert_eq!(profiles.len(), 1);
    let profile = &profiles[0];
    assert_eq!(profile.id, "user123");
    assert_eq!(profile.attributes["name"], "Alice");
    std::fs::remove_file("dummy.csv").unwrap();
}

#[test]
fn test_profile_processing() {
    let profiles = vec![CustomerProfile {
        id: "user123".to_string(),
        attributes: json!({"name": "Alice"}),
    }];
    let processed_profiles = process_profiles(profiles);
    assert_eq!(processed_profiles.len(), 1);
    let profile = &processed_profiles[0];
    assert!(profile.attributes.get("processed_at").is_some());
}

#[test]
fn test_segmentation() {
    let profiles = vec![
        CustomerProfile {
            id: "user1".to_string(),
            attributes: json!({"city": "New York", "spent": 200}),
        },
        CustomerProfile {
            id: "user2".to_string(),
            attributes: json!({"city": "Los Angeles", "spent": 80}),
        },
        CustomerProfile {
            id: "user3".to_string(),
            attributes: json!({"city": "New York", "spent": 50}),
        },
    ];

    let segments = vec![
        Segment {
            id: "high_spenders".to_string(),
            name: "High Spenders".to_string(),
            rule: Box::new(|p| p.attributes["spent"].as_i64().unwrap_or(0) > 100),
        },
        Segment {
            id: "ny_customers".to_string(),
            name: "New York Customers".to_string(),
            rule: Box::new(|p| p.attributes["city"] == "New York"),
        },
    ];

    let segmented = segment_profiles(&profiles, &segments);

    assert_eq!(segmented.len(), 2);

    let high_spenders = segmented
        .iter()
        .find(|(id, _)| id == "high_spenders")
        .unwrap();
    assert_eq!(high_spenders.1, vec!["user1"]);

    let ny_customers = segmented
        .iter()
        .find(|(id, _)| id == "ny_customers")
        .unwrap();
    assert_eq!(ny_customers.1, vec!["user1", "user3"]);
}

#[test]
fn test_identity_resolution() {
    let profiles = vec![
        CustomerProfile {
            id: "user123".to_string(),
            attributes: json!({"email": "alice@example.com", "device": "mobile"}),
        },
        CustomerProfile {
            id: "user456".to_string(),
            attributes: json!({"email": "alice@example.com", "device": "desktop"}),
        },
    ];

    let resolved_profile = resolve_identities(profiles).unwrap();
    assert_eq!(resolved_profile.id, "user123");
    assert_eq!(
        resolved_profile.attributes,
        json!({
            "email": "alice@example.com",
            "device": "desktop" // Last write wins
        })
    );
}

#[test]
fn test_identity_resolution_empty() {
    let profiles = vec![];
    assert!(resolve_identities(profiles).is_none());
}

#[test]
fn test_activation() {
    let profile = CustomerProfile {
        id: "user789".to_string(),
        attributes: json!({"name": "Bob"}),
    };

    let email_target = ActivationTarget::Email("bob@example.com".to_string());
    assert!(activate_profile(&profile, &email_target).is_ok());

    let push_target = ActivationTarget::PushNotification("push-token-123".to_string());
    assert!(activate_profile(&profile, &push_target).is_ok());

    let webhook_target = ActivationTarget::Webhook("https://example.com/webhook".to_string());
    assert!(activate_profile(&profile, &webhook_target).is_ok());
}
