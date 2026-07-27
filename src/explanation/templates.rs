use crate::detection::observations::{BehaviorObservation, MostChangedEntity, ObservationCategory, ScoredEvent};
use crate::domain::Entity;

pub struct ExplanationTemplates;

impl ExplanationTemplates {
    /// Format a single observation into grounded natural language
    pub fn format_observation(obs: &BehaviorObservation) -> String {
        let time_str = obs.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        match obs.category {
            ObservationCategory::NewBehavior => {
                format!(
                    "At {}, entity '{}' exhibited a brand new behavior ('{}'). Anomaly score: {:.2}. {}",
                    time_str, obs.entity_canonical_name, obs.title, obs.anomaly_score, obs.description
                )
            }
            ObservationCategory::BehaviorDrift => {
                format!(
                    "Entity '{}' shows significant behavioral drift (drift score: {:.2}). {}",
                    obs.entity_canonical_name, obs.anomaly_score, obs.description
                )
            }
            ObservationCategory::SuddenAnomaly => {
                format!(
                    "At {}, entity '{}' triggered a sudden behavioral anomaly ('{}'). Score: {:.2}. {}",
                    time_str, obs.entity_canonical_name, obs.title, obs.anomaly_score, obs.description
                )
            }
            ObservationCategory::StableBehavior => {
                format!(
                    "Entity '{}' maintains stable, expected behavior patterns across baselines.",
                    obs.entity_canonical_name
                )
            }
        }
    }

    /// Format a ScoredEvent into detailed natural language evidence
    pub fn format_scored_event(scored: &ScoredEvent, entity: &Entity) -> String {
        let time_str = scored.event.timestamp.format("%H:%M:%S UTC").to_string();
        let target_desc = match &scored.event.target_id {
            Some(t_id) => format!(" target entity [{}]", t_id),
            None => "".to_string(),
        };

        if scored.graph_score == 1.0 {
            format!(
                "Entity '{}' performed action '{}' against{} at {}. This target entity has not appeared in this entity's connection history (Graph Anomaly: 1.00, Fused Score: {:.2}).",
                entity.canonical_name, scored.event.event_type, target_desc, time_str, scored.fused_score
            )
        } else if scored.velocity_score > 0.7 {
            format!(
                "Entity '{}' executed action '{}' at an unusually high rate at {} (Velocity Anomaly: {:.2}, Fused Score: {:.2}).",
                entity.canonical_name, scored.event.event_type, time_str, scored.velocity_score, scored.fused_score
            )
        } else {
            format!(
                "Entity '{}' executed action '{}' at {} with fused anomaly score {:.2} (Sequence: {:.2}, Velocity: {:.2}, Graph: {:.2}).",
                entity.canonical_name, scored.event.event_type, time_str, scored.fused_score, scored.sequence_score, scored.velocity_score, scored.graph_score
            )
        }
    }

    /// Format daily narrative briefing for `lafufu explain`
    pub fn format_daily_briefing(
        window_str: &str,
        observations: &[BehaviorObservation],
        most_changed: &[MostChangedEntity],
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!("========================================\n"));
        out.push_str(&format!(" Lafufu Behavioral Intelligence Briefing\n"));
        out.push_str(&format!(" Window: {}\n", window_str));
        out.push_str(&format!("========================================\n\n"));

        out.push_str(&format!("1. Executive Summary:\n"));
        if observations.is_empty() && most_changed.is_empty() {
            out.push_str("   All tracked entities show consistent, stable behavior. No anomalous drift or unexpected events observed.\n\n");
            return out;
        }

        out.push_str(&format!(
            "   Observed {} behavioral changes requiring review. {} entities showed significant profile drift.\n\n",
            observations.len(),
            most_changed.len()
        ));

        if !most_changed.is_empty() {
            out.push_str("2. Top Entities With Significant Behavioral Change:\n");
            for (i, item) in most_changed.iter().enumerate() {
                out.push_str(&format!(
                    "   {}. {} ({}) — Change Score: {:.2} (Drift: {:.2}, Anomaly: {:.2})\n",
                    i + 1, item.canonical_name, item.entity_type, item.combined_change_score, item.drift_score, item.anomaly_score
                ));
            }
            out.push('\n');
        }

        if !observations.is_empty() {
            out.push_str("3. Key Observations & Findings:\n");
            for obs in observations {
                out.push_str(&format!("   • {}\n", Self::format_observation(obs)));
            }
            out.push('\n');
        }

        out.push_str("========================================\n");
        out
    }
}
