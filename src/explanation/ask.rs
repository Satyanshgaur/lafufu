use crate::detection::engine::DetectionEngine;
use crate::repository::EntityRepository;
use crate::storage::sqlite::SqliteStorage;
use chrono::{Duration, Utc};

pub struct ConversationalQueryEngine;

impl ConversationalQueryEngine {
    /// Answer user natural language queries grounded in behavior graph evidence
    pub fn answer_question(
        query: &str,
        storage: &SqliteStorage,
        detection_engine: &DetectionEngine,
    ) -> crate::errors::Result<String> {
        let entities = EntityRepository::find_all(storage)?;
        let observations = detection_engine.detect_observations(Utc::now() - Duration::days(7))?;
        let most_changed = detection_engine.get_most_changed_entities(5)?;

        let query_lower = query.to_lowercase();

        let mut out = String::new();
        out.push_str(&format!("========================================\n"));
        out.push_str(&format!(" Lafufu Conversational Query Response\n"));
        out.push_str(&format!(" Query: \"{}\"\n", query));
        out.push_str(&format!("========================================\n\n"));

        if query_lower.contains("change") || query_lower.contains("what happened") || query_lower.contains("anomaly") {
            out.push_str("Based on behavior graph evidence for the past 7 days:\n\n");
            if observations.is_empty() && most_changed.is_empty() {
                out.push_str("No significant behavioral changes or anomalies detected across any active entity baselines.\n");
            } else {
                if !most_changed.is_empty() {
                    out.push_str("Entities with highest behavioral change:\n");
                    for item in &most_changed {
                        out.push_str(&format!(
                            " • {} ({}): Change Score {:.2} (Drift: {:.2}, Anomaly: {:.2})\n",
                            item.canonical_name, item.entity_type, item.combined_change_score, item.drift_score, item.anomaly_score
                        ));
                    }
                    out.push('\n');
                }

                if !observations.is_empty() {
                    out.push_str("Key Behavioral Observations:\n");
                    for obs in &observations {
                        out.push_str(&format!(" • [{:?}] {}: {}\n", obs.category, obs.entity_canonical_name, obs.description));
                    }
                }
            }
        } else {
            // General query response
            out.push_str(&format!("Tracking {} total entities in the local behavior graph.\n\n", entities.len()));
            out.push_str("Active Entity Roster:\n");
            for entity in &entities {
                out.push_str(&format!(" • {} ({})\n", entity.canonical_name, entity.entity_type));
            }
        }

        out.push_str("\n========================================\n");
        Ok(out)
    }
}
