use serde::{Deserialize, Serialize};

/// Weight parameters for fusing sequence, velocity, and graph signals into a composite anomaly score
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct FusionWeights {
    pub weight_sequence: f64,
    pub weight_velocity: f64,
    pub weight_graph: f64,
}

impl FusionWeights {
    pub fn new(seq: f64, vel: f64, graph: f64) -> Self {
        let total = seq + vel + graph;
        if total == 0.0 {
            Self::default()
        } else {
            Self {
                weight_sequence: seq / total,
                weight_velocity: vel / total,
                weight_graph: graph / total,
            }
        }
    }

    /// Default fusion weights tailored to entity types
    pub fn for_entity_type(entity_type: &str) -> Self {
        match entity_type.to_lowercase().as_str() {
            "user" => Self::new(0.40, 0.20, 0.40),
            "service" => Self::new(0.20, 0.40, 0.40),
            "container" => Self::new(0.20, 0.30, 0.50),
            "github_repo" | "repository" | "repo" => Self::new(0.30, 0.30, 0.40),
            _ => Self::default(),
        }
    }
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            weight_sequence: 0.33,
            weight_velocity: 0.33,
            weight_graph: 0.34,
        }
    }
}

pub struct FusionEngine;

impl FusionEngine {
    /// Fuse sequence, velocity, and graph anomaly scores using entity-specific weights
    pub fn fuse(seq_score: f64, vel_score: f64, graph_score: f64, weights: &FusionWeights) -> f64 {
        let score = weights.weight_sequence * seq_score
            + weights.weight_velocity * vel_score
            + weights.weight_graph * graph_score;

        score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_weight_normalization() {
        let weights = FusionWeights::new(2.0, 2.0, 4.0);
        assert_eq!(weights.weight_sequence, 0.25);
        assert_eq!(weights.weight_velocity, 0.25);
        assert_eq!(weights.weight_graph, 0.50);
    }

    #[test]
    fn test_fusion_score() {
        let weights = FusionWeights::for_entity_type("user");
        let score = FusionEngine::fuse(1.0, 0.0, 1.0, &weights);
        // User weights: 0.40 * 1.0 + 0.20 * 0.0 + 0.40 * 1.0 = 0.80
        assert!((score - 0.80).abs() < 1e-5);
    }
}
