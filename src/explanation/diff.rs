use crate::behavior::engine::BaselineEngine;
use crate::behavior::profile::Profile;
use crate::domain::Entity;
use crate::errors::{LafufuError, Result};

pub struct ProfileDiffEngine;

impl ProfileDiffEngine {
    /// Compare an entity's short-term / medium-term baseline vs long-term baseline profile
    pub fn diff_entity_profiles(
        entity: &Entity,
        baseline_engine: &BaselineEngine,
    ) -> Result<String> {
        let layers = baseline_engine.load_temporal_layers(&entity.id)?
            .ok_or_else(|| LafufuError::Analysis(format!("No baselines computed for entity {}", entity.canonical_name)))?;

        let mut out = String::new();
        out.push_str(&format!("========================================\n"));
        out.push_str(&format!(" Behavioral Profile Diff: {}\n", entity.canonical_name));
        out.push_str(&format!("========================================\n"));

        let drift = layers.calculate_drift();
        let anomaly = layers.calculate_anomaly();

        out.push_str(&format!("Temporal Horizon Divergence Metrics:\n"));
        out.push_str(&format!(" • Medium-term vs Long-term Drift Score: {:.4}\n", drift));
        out.push_str(&format!(" • Short-term vs Long-term Anomaly Score: {:.4}\n\n", anomaly));

        match (&layers.medium_term, &layers.long_term) {
            (Profile::User(p_med), Profile::User(p_long)) => {
                out.push_str("Shift in User Action Frequency (Medium-term vs Long-term):\n");
                for (act, weight) in &p_med.actions.bins {
                    let long_weight = p_long.actions.bins.get(act).copied().unwrap_or(0.0);
                    out.push_str(&format!("   - {}: medium = {:.1}, long-term = {:.1}\n", act, weight, long_weight));
                }
            }
            (Profile::Service(p_med), Profile::Service(p_long)) => {
                out.push_str("Shift in Service Action Frequency (Medium-term vs Long-term):\n");
                for (act, weight) in &p_med.action_frequencies.bins {
                    let long_weight = p_long.action_frequencies.bins.get(act).copied().unwrap_or(0.0);
                    out.push_str(&format!("   - {}: medium = {:.1}, long-term = {:.1}\n", act, weight, long_weight));
                }
            }
            _ => {
                out.push_str("Generic profile shift analysis complete.\n");
            }
        }

        out.push_str("========================================\n");
        Ok(out)
    }
}
