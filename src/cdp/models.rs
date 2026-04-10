//! src/cdp/models.rs
//!
//! Core data structures for the Customer Data Platform (CDP) framework.

use std::collections::HashMap;

/// Represents a value that can be stored in a customer profile or event.
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    // Future additions could include DateTime, Array, etc.
}

/// Represents a single customer event.
///
/// Events are the raw data points ingested into the CDP, such as page views,
/// transactions, or support tickets.
#[derive(Debug, Clone)]
pub struct Event {
    pub event_id: String,
    pub customer_id: String,
    pub event_type: String,
    pub timestamp: u64, // Unix timestamp
    pub properties: HashMap<String, DataValue>,
}

/// Represents a unified customer profile.
///
/// This structure holds the aggregated and unified data from various events
/// for a single customer. It forms the "single source of truth" for a customer.
#[derive(Debug, Clone, Default)]
pub struct CustomerProfile {
    pub customer_id: String,
    pub attributes: HashMap<String, DataValue>,
    // A log of events that contributed to this profile.
    // In a real high-performance scenario, this might be an external link
    // to a data warehouse or event store.
    pub event_history: Vec<String>,
}

impl CustomerProfile {
    pub fn new(customer_id: String) -> Self {
        Self {
            customer_id,
            attributes: HashMap::new(),
            event_history: Vec::new(),
        }
    }
}

/// Represents a segment of customers.
///
/// Segments are dynamic groups of customers defined by a set of rules
/// or criteria.
#[derive(Debug)]
pub struct Segment {
    pub segment_id: String,
    pub name: String,
    // The rules defining the segment would be more complex in a real system,
    // potentially using a dedicated rules engine DSL.
    // For this example, we'll keep it simple.
    pub rules: Vec<SegmentRule>,
}

/// A single rule for defining a segment.
#[derive(Debug, Clone)]
pub struct SegmentRule {
    pub attribute: String,
    pub operator: RuleOperator,
    pub value: DataValue,
}

#[derive(Debug, Clone)]
pub enum RuleOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains, // For string values
}
