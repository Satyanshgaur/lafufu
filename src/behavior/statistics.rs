use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An exponentially weighted moving average for numeric tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ewma {
    pub value: f64,
    pub alpha: f64, // Decay factor, typically between 0.0 and 1.0 (e.g., 0.1)
}

impl Ewma {
    pub fn new(initial_value: f64, alpha: f64) -> Self {
        Self {
            value: initial_value,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    pub fn update(&mut self, next_value: f64) {
        self.value = self.alpha * next_value + (1.0 - self.alpha) * self.value;
    }
}

/// A simple categorical histogram for tracking frequencies of events (e.g. login hours, ports, commands).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CategoricalHistogram {
    pub bins: HashMap<String, u64>,
    pub total_count: u64,
}

impl CategoricalHistogram {
    pub fn new() -> Self {
        Self {
            bins: HashMap::new(),
            total_count: 0,
        }
    }

    pub fn observe(&mut self, category: &str) {
        *self.bins.entry(category.to_string()).or_insert(0) += 1;
        self.total_count += 1;
    }

    pub fn probability(&self, category: &str) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        let count = self.bins.get(category).copied().unwrap_or(0);
        count as f64 / self.total_count as f64
    }
}
