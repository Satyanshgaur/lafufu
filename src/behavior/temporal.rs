use crate::behavior::profile::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemporalLayers {
    pub short_term: Profile, // 24-72h baseline
    pub medium_term: Profile, // 7-30d baseline
    pub long_term: Profile, // 90-365d baseline
}

impl TemporalLayers {
    pub fn new(short: Profile, medium: Profile, long: Profile) -> Self {
        Self {
            short_term: short,
            medium_term: medium,
            long_term: long,
        }
    }
}
