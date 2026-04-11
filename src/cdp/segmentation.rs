// src/cdp/segmentation.rs

use super::CustomerProfile;

pub struct Segment {
    pub id: String,
    pub name: String,
    // A function that determines if a profile belongs to the segment
    pub rule: Box<dyn Fn(&CustomerProfile) -> bool>,
}

pub fn segment_profiles(
    profiles: &[CustomerProfile],
    segments: &[Segment],
) -> Vec<(String, Vec<String>)> {
    segments
        .iter()
        .map(|segment| {
            let profile_ids = profiles
                .iter()
                .filter(|profile| (segment.rule)(profile))
                .map(|profile| profile.id.clone())
                .collect();
            (segment.id.clone(), profile_ids)
        })
        .collect()
}
