use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exponentially Weighted Moving Average (EWMA) for continuous numeric tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ewma {
    pub value: f64,
    pub alpha: f64, // Decay factor between 0.0 and 1.0
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

/// Running statistics tracker for calculating mean, variance, and standard deviation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RunningStats {
    pub count: u64,
    pub mean: f64,
    pub m2: f64, // Sum of squared differences from mean (Welford's algorithm)
}

impl RunningStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn z_score(&self, x: f64) -> f64 {
        let std = self.std_dev();
        if std == 0.0 {
            if (x - self.mean).abs() < 1e-9 {
                0.0
            } else {
                (x - self.mean).abs()
            }
        } else {
            (x - self.mean) / std
        }
    }
}

/// Categorical Histogram with frequency counts, Laplace-smoothed probability estimation,
/// and Jensen-Shannon Divergence calculation for distribution comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CategoricalHistogram {
    pub bins: HashMap<String, f64>,
    pub total_weight: f64,
}

impl CategoricalHistogram {
    pub fn new() -> Self {
        Self {
            bins: HashMap::new(),
            total_weight: 0.0,
        }
    }

    pub fn observe(&mut self, category: &str) {
        self.observe_weight(category, 1.0);
    }

    pub fn observe_weight(&mut self, category: &str, weight: f64) {
        *self.bins.entry(category.to_string()).or_insert(0.0) += weight;
        self.total_weight += weight;
    }

    /// Calculate raw probability P(category)
    pub fn raw_probability(&self, category: &str) -> f64 {
        if self.total_weight == 0.0 {
            return 0.0;
        }
        let count = self.bins.get(category).copied().unwrap_or(0.0);
        count / self.total_weight
    }

    /// Calculate Laplace-smoothed probability estimation to handle zero-frequency events
    pub fn smoothed_probability(&self, category: &str) -> f64 {
        let vocab_size = (self.bins.len() + 1) as f64; // +1 for unseen categories
        let count = self.bins.get(category).copied().unwrap_or(0.0);
        (count + 1.0) / (self.total_weight + vocab_size)
    }

    /// Calculate Jensen-Shannon Divergence (JSD) between two histograms [0.0 = identical, 1.0 = completely divergent]
    pub fn jensen_shannon_divergence(&self, other: &CategoricalHistogram) -> f64 {
        if self.total_weight == 0.0 && other.total_weight == 0.0 {
            return 0.0;
        }

        // Collect union of all categories
        let mut keys: std::collections::HashSet<&String> = self.bins.keys().collect();
        keys.extend(other.bins.keys());

        if keys.is_empty() {
            return 0.0;
        }

        let mut kl_p_m = 0.0;
        let mut kl_q_m = 0.0;

        for key in keys {
            let p = self.smoothed_probability(key);
            let q = other.smoothed_probability(key);
            let m = 0.5 * (p + q);

            kl_p_m += p * (p / m).ln();
            kl_q_m += q * (q / m).ln();
        }

        let jsd = 0.5 * kl_p_m + 0.5 * kl_q_m;
        // Normalize and clamp between 0.0 and 1.0 using LN_2 constant
        (jsd / std::f64::consts::LN_2).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_stats() {
        let mut stats = RunningStats::new();
        stats.update(10.0);
        stats.update(20.0);
        stats.update(30.0);

        assert_eq!(stats.mean, 20.0);
        assert!((stats.std_dev() - 10.0).abs() < 1e-5);
        assert_eq!(stats.z_score(40.0), 2.0);
    }

    #[test]
    fn test_categorical_histogram_jsd() {
        let mut h1 = CategoricalHistogram::new();
        h1.observe("login");
        h1.observe("login");

        let mut h2 = CategoricalHistogram::new();
        h2.observe("login");
        h2.observe("logout");

        let jsd = h1.jensen_shannon_divergence(&h2);
        assert!(jsd > 0.0 && jsd < 1.0);

        let self_jsd = h1.jensen_shannon_divergence(&h1);
        assert!(self_jsd < 1e-5);
    }
}
