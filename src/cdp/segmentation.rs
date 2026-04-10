//! src/cdp/segmentation.rs
//!
//! Handles the logic for grouping customer profiles into segments based on defined rules.

use crate::cdp::models::{CustomerProfile, DataValue, RuleOperator, Segment, SegmentRule};

/// The SegmentEngine evaluates profiles against segment rules.
pub struct SegmentEngine;

impl SegmentEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates a single profile against a segment's rules to determine membership.
    pub fn evaluate_profile(&self, profile: &CustomerProfile, segment: &Segment) -> bool {
        // A profile must match ALL rules in a segment (AND logic).
        // A more advanced engine could support OR logic or nested conditions.
        segment
            .rules
            .iter()
            .all(|rule| self.evaluate_rule(profile, rule))
    }

    /// Evaluates a single rule against a profile.
    fn evaluate_rule(&self, profile: &CustomerProfile, rule: &SegmentRule) -> bool {
        match profile.attributes.get(&rule.attribute) {
            Some(profile_value) => self.compare_values(profile_value, &rule.operator, &rule.value),
            None => false, // Attribute doesn't exist on the profile.
        }
    }

    /// Compares a profile's attribute value with a rule's value using the given operator.
    fn compare_values(&self, a: &DataValue, op: &RuleOperator, b: &DataValue) -> bool {
        match (op, a, b) {
            (RuleOperator::Equals, _, _) => a == b,
            (RuleOperator::NotEquals, _, _) => a != b,
            (RuleOperator::GreaterThan, DataValue::Int(val_a), DataValue::Int(val_b)) => val_a > val_b,
            (RuleOperator::GreaterThan, DataValue::Float(val_a), DataValue::Float(val_b)) => val_a > val_b,
            (RuleOperator::LessThan, DataValue::Int(val_a), DataValue::Int(val_b)) => val_a < val_b,
            (RuleOperator::LessThan, DataValue::Float(val_a), DataValue::Float(val_b)) => val_a < val_b,
            (RuleOperator::Contains, DataValue::String(val_a), DataValue::String(val_b)) => val_a.contains(val_b),
            // Return false for type mismatches or unsupported operations.
            _ => false,
        }
    }

    /// Finds all profiles that belong to a given segment.
    pub fn get_segment_members<'a>(
        &self,
        profiles: Vec<&'a CustomerProfile>,
        segment: &Segment,
    ) -> Vec<&'a CustomerProfile> {
        profiles
            .into_iter()
            .filter(|p| self.evaluate_profile(p, segment))
            .collect()
    }
}

impl Default for SegmentEngine {
    fn default() -> Self {
        Self::new()
    }
}
