// tests/cdp_framework.rs

use ny::cdp::{
    activation::{ActivationTarget, MockApiTarget},
    identity::resolve_identities,
    ingestion::{CsvSource, IngestionSource},
    processing::process_profiles,
    segmentation::{segment_profiles, Segment},
    CustomerProfile,
};
use serde_json::json;

#[test]
fn test_csv_ingestion() {
    let source = CsvSource { path: "dummy.csv" };
    let profiles = source.ingest().unwrap();
    assert_eq!(profiles.len(), 1);
    let profile = &profiles[0];
    assert_eq!(profile.id, "user123");
    assert_eq!(profile.attributes["name"], "Alice");
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
fn test_identity_resolution() {
    let profiles = vec![
        CustomerProfile {
            id: "user1".to_string(),
            attributes: json!({"email": "alice@example.com", "first_name": "Alice"}),
        },
        CustomerProfile {
            id: "user2".to_string(),
            attributes: json!({"email": "bob@example.com", "first_name": "Bob"}),
        },
        CustomerProfile {
            id: "user3".to_string(),
            attributes: json!({"email": "alice@example.com", "last_name": "Smith"}),
        },
    ];

    let resolved_profiles = resolve_identities(profiles, "email");
    assert_eq!(resolved_profiles.len(), 2);

    let alice_profile = resolved_profiles
        .iter()
        .find(|p| p.attributes["email"] == "alice@example.com")
        .unwrap();
    assert_eq!(alice_profile.attributes["first_name"], "Alice");
    assert_eq!(alice_profile.attributes["last_name"], "Smith");

    let bob_profile = resolved_profiles
        .iter()
        .find(|p| p.attributes["email"] == "bob@example.com")
        .unwrap();
    assert_eq!(bob_profile.attributes["first_name"], "Bob");
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
fn test_activation() {
    let profiles = vec![CustomerProfile {
        id: "user1".to_string(),
        attributes: json!({"email": "test@example.com"}),
    }];
    let target = MockApiTarget;
    let result = target.activate(&profiles);
    assert!(result.is_ok());
}
