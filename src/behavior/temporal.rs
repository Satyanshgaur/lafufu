use crate::behavior::profile::Profile;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Time window configurations for the three temporal baseline layers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TemporalWindows {
    pub short_term_hours: i64,  // Default: 72h (3 days)
    pub medium_term_days: i64,  // Default: 30 days
    pub long_term_days: i64,    // Default: 365 days
}

impl Default for TemporalWindows {
    fn default() -> Self {
        Self {
            short_term_hours: 72,
            medium_term_days: 30,
            long_term_days: 365,
        }
    }
}

impl TemporalWindows {
    pub fn short_term_since(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::hours(self.short_term_hours)
    }

    pub fn medium_term_since(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::days(self.medium_term_days)
    }

    pub fn long_term_since(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::days(self.long_term_days)
    }
}

/// Representation of an entity's behavior across three temporal horizons simultaneously
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemporalLayers {
    pub short_term: Profile,  // 24-72h baseline (immediate spikes/anomalies)
    pub medium_term: Profile, // 7-30d baseline (gradual drift)
    pub long_term: Profile,   // 90-365d baseline (stable foundation)
}

impl TemporalLayers {
    pub fn new(short: Profile, medium: Profile, long: Profile) -> Self {
        Self {
            short_term: short,
            medium_term: medium,
            long_term: long,
        }
    }

    /// Measure behavioral drift (medium-term vs long-term baseline divergence) [0.0 = no drift, 1.0 = heavy drift]
    pub fn calculate_drift(&self) -> f64 {
        self.medium_term.jensen_shannon_divergence(&self.long_term)
    }

    /// Measure immediate anomaly (short-term vs long-term baseline divergence) [0.0 = normal, 1.0 = high anomaly]
    pub fn calculate_anomaly(&self) -> f64 {
        self.short_term.jensen_shannon_divergence(&self.long_term)
    }
}
