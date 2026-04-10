// Allow dead code for modules that are not yet fully integrated.
#![allow(dead_code)]

// Main test suite for the CDP Framework
mod cdp_framework {
    use ny_lang::cdp::{
        activation::{ActivationTarget, MockTarget},
        ingestion::{IngestionStream, MockSource},
        models::{
            CustomerProfile, DataValue, Event, RuleOperator, Segment, SegmentRule,
        },
        processing::ProfileUnificationEngine,
        segmentation::SegmentEngine,
    };
    use std::collections::HashMap;

    fn create_mock_event(customer_id: &str, event_type: &str, properties: HashMap<String, DataValue>) -> Event {
        Event {
            event_id: uuid::Uuid::new_v4().to_string(),
            customer_id: customer_id.to_string(),
            event_type: event_type.to_string(),
            timestamp: 1678886400, // A fixed timestamp for predictability
            properties,
        }
    }

    #[test]
    fn test_ingestion_stream() {
        let (stream, receiver) = IngestionStream::new();
        let event = create_mock_event("cust_123", "page_view", HashMap::new());
        let event_id = event.event_id.clone();

        stream.push(event).unwrap();

        let received_event = receiver.recv().unwrap();
        assert_eq!(received_event.event_id, event_id);
        assert_eq!(received_event.customer_id, "cust_123");
    }

    #[test]
    fn test_profile_unification() {
        let mut engine = ProfileUnificationEngine::new();
        let mut props = HashMap::new();
        props.insert("page_url".to_string(), DataValue::String("/homepage".to_string()));
        let event1 = create_mock_event("cust_456", "page_view", props);

        let mut props2 = HashMap::new();
        props2.insert("item_id".to_string(), DataValue::String("item_abc".to_string()));
        props2.insert("price".to_string(), DataValue::Float(99.99));
        let event2 = create_mock_event("cust_456", "purchase", props2);

        engine.process_event(&event1);
        engine.process_event(&event2);

        let profile = engine.get_profile("cust_456").unwrap();
        assert_eq!(profile.customer_id, "cust_456");
        assert_eq!(profile.event_history.len(), 2);
        assert_eq!(
            profile.attributes.get("page_url"),
            Some(&DataValue::String("/homepage".to_string()))
        );
        assert_eq!(
            profile.attributes.get("price"),
            Some(&DataValue::Float(99.99))
        );
        assert!(profile.attributes.contains_key("last_seen_ts"));
    }

    #[test]
    fn test_segmentation_engine() {
        let segment_engine = SegmentEngine::new();

        // Segment: "High-Value Customers" - total_spend > 1000
        let high_value_segment = Segment {
            segment_id: "seg_1".to_string(),
            name: "High-Value Customers".to_string(),
            rules: vec![SegmentRule {
                attribute: "total_spend".to_string(),
                operator: RuleOperator::GreaterThan,
                value: DataValue::Int(1000),
            }],
        };

        let mut profile1 = CustomerProfile::new("cust_1".to_string());
        profile1.attributes.insert("total_spend".to_string(), DataValue::Int(1500));

        let mut profile2 = CustomerProfile::new("cust_2".to_string());
        profile2.attributes.insert("total_spend".to_string(), DataValue::Int(500));
        
        let mut profile3 = CustomerProfile::new("cust_3".to_string());
        profile3.attributes.insert("email_domain".to_string(), DataValue::String("work.com".to_string()));


        assert!(segment_engine.evaluate_profile(&profile1, &high_value_segment));
        assert!(!segment_engine.evaluate_profile(&profile2, &high_value_segment));
        assert!(!segment_engine.evaluate_profile(&profile3, &high_value_segment));

        let all_profiles = vec![&profile1, &profile2, &profile3];
        let members = segment_engine.get_segment_members(all_profiles, &high_value_segment);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].customer_id, "cust_1");
    }
    
    #[test]
    fn test_segmentation_with_string_contains() {
        let segment_engine = SegmentEngine::new();

        // Segment: "Corporate Users" - email contains "@company.com"
        let corporate_segment = Segment {
            segment_id: "seg_2".to_string(),
            name: "Corporate Users".to_string(),
            rules: vec![SegmentRule {
                attribute: "email".to_string(),
                operator: RuleOperator::Contains,
                value: DataValue::String("@company.com".to_string()),
            }],
        };

        let mut profile1 = CustomerProfile::new("user_A".to_string());
        profile1.attributes.insert("email".to_string(), DataValue::String("test@company.com".to_string()));

        let mut profile2 = CustomerProfile::new("user_B".to_string());
        profile2.attributes.insert("email".to_string(), DataValue::String("user@gmail.com".to_string()));

        assert!(segment_engine.evaluate_profile(&profile1, &corporate_segment));
        assert!(!segment_engine.evaluate_profile(&profile2, &corporate_segment));
    }


    #[test]
    fn test_activation_target() {
        let mock_target = MockTarget::new("Test-Email-Campaign", false);
        let profile1 = CustomerProfile::new("cust_abc".to_string());
        let profile2 = CustomerProfile::new("cust_def".to_string());
        let profiles_to_activate = vec![&profile1, &profile2];

        let result = mock_target.activate(profiles_to_activate).unwrap();

        assert_eq!(result.target_name, "Test-Email-Campaign");
        assert_eq!(result.successful_activations, 2);
        assert_eq!(result.failed_activations, 0);
    }
    
    #[test]
    fn test_failed_activation() {
        let mock_target_fail = MockTarget::new("Failing-Target", true);
        let profile1 = CustomerProfile::new("cust_xyz".to_string());
        let profiles_to_activate = vec![&profile1];

        let result = mock_target_fail.activate(profiles_to_activate);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_cdp_flow() {
        // 1. Ingestion
        let (stream, receiver) = IngestionStream::new();
        let mut props = HashMap::new();
        props.insert("total_spend".to_string(), DataValue::Int(1200));
        props.insert("last_login_date".to_string(), DataValue::String("2023-10-26".to_string()));
        let event1 = create_mock_event("customer_A", "purchase", props);

        let mut props2 = HashMap::new();
        props2.insert("total_spend".to_string(), DataValue::Int(300));
        let event2 = create_mock_event("customer_B", "purchase", props2);

        stream.push(event1).unwrap();
        stream.push(event2).unwrap();

        // 2. Processing
        let mut unification_engine = ProfileUnificationEngine::new();
        for _ in 0..2 {
            let event = receiver.recv().unwrap();
            unification_engine.process_event(&event);
        }

        // 3. Segmentation
        let segment_engine = SegmentEngine::new();
        let high_value_segment = Segment {
            segment_id: "seg_high_value".to_string(),
            name: "High Value Customers".to_string(),
            rules: vec![SegmentRule {
                attribute: "total_spend".to_string(),
                operator: RuleOperator::GreaterThan,
                value: DataValue::Int(1000),
            }],
        };
        let all_profiles = unification_engine.get_all_profiles();
        let high_value_members = segment_engine.get_segment_members(all_profiles, &high_value_segment);

        assert_eq!(high_value_members.len(), 1);
        assert_eq!(high_value_members[0].customer_id, "customer_A");

        // 4. Activation
        let email_target = MockTarget::new("High-Value-Email-List", false);
        let result = email_target.activate(high_value_members).unwrap();
        assert_eq!(result.successful_activations, 1);
    }
}
