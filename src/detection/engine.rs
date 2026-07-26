use crate::behavior::engine::BaselineEngine;
use crate::behavior::profile::Profile;
use crate::behavior::statistics::RunningStats;
use crate::detection::fusion::{FusionEngine, FusionWeights};
use crate::detection::observations::{BehaviorObservation, MostChangedEntity, ObservationCategory, ScoredEvent};
use crate::detection::signals::{GraphSignal, SequenceSignal, VelocitySignal};
use crate::domain::{Entity, Event};
use crate::errors::Result;
use crate::repository::{EntityRepository, EventRepository};
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

pub struct DetectionEngine {
    storage: SqliteStorage,
    baseline_engine: Arc<BaselineEngine>,
}

impl DetectionEngine {
    pub fn new(storage: SqliteStorage, baseline_engine: Arc<BaselineEngine>) -> Self {
        Self {
            storage,
            baseline_engine,
        }
    }

    /// Score a single event against the entity's baseline profile
    pub fn score_event(&self, event: &Event, entity: &Entity) -> Result<ScoredEvent> {
        let layers = self.baseline_engine.load_temporal_layers(&entity.id)?;
        let profile = layers
            .map(|l| l.long_term)
            .unwrap_or_else(|| Profile::for_entity_type(&entity.entity_type));

        // 1. Sequence Signal Score
        let seq_score = SequenceSignal::score(event, &profile);

        // 2. Velocity Signal Score
        let recent_events = EventRepository::find_by_entity(
            &self.storage,
            &entity.id,
            event.timestamp - Duration::hours(1),
        )?;
        let mut stats = RunningStats::new();
        stats.update(recent_events.len() as f64);
        let vel_score = VelocitySignal::score(recent_events.len() as f64, &stats);

        // 3. Graph Signal Score
        let target_str = event.target_id.map(|id| id.to_string());
        let graph_score = GraphSignal::score(target_str.as_deref(), None, &profile);

        // 4. Fused Anomaly Score
        let weights = FusionWeights::for_entity_type(&entity.entity_type);
        let fused_score = FusionEngine::fuse(seq_score, vel_score, graph_score, &weights);

        Ok(ScoredEvent {
            event: event.clone(),
            sequence_score: seq_score,
            velocity_score: vel_score,
            graph_score,
            fused_score,
        })
    }

    /// Detect behavioral observations across recent events
    pub fn detect_observations(&self, since: DateTime<Utc>) -> Result<Vec<BehaviorObservation>> {
        let events = EventRepository::find_all_since(&self.storage, since)?;
        let entities = EntityRepository::find_all(&self.storage)?;
        let mut observations = Vec::new();

        for entity in &entities {
            let entity_events: Vec<&Event> = events.iter().filter(|e| e.source_id == entity.id).collect();
            if entity_events.is_empty() {
                continue;
            }

            if let Some(layers) = self.baseline_engine.load_temporal_layers(&entity.id)? {
                let drift_score = layers.calculate_drift();
                let _anomaly_score = layers.calculate_anomaly();

                // Check for Behavior Drift
                if drift_score > 0.35 {
                    observations.push(BehaviorObservation {
                        entity_id: entity.id,
                        entity_canonical_name: entity.canonical_name.clone(),
                        category: ObservationCategory::BehaviorDrift,
                        title: format!("Behavior drift detected in {}", entity.canonical_name),
                        description: format!(
                            "Entity {} shows gradual behavioral drift (medium vs long term divergence: {:.2})",
                            entity.canonical_name, drift_score
                        ),
                        anomaly_score: drift_score,
                        timestamp: Utc::now(),
                    });
                }

                // Evaluate individual events for New Behaviors or Sudden Anomalies
                for ev in entity_events {
                    let scored = self.score_event(ev, entity)?;

                    if scored.fused_score > 0.70 {
                        observations.push(BehaviorObservation {
                            entity_id: entity.id,
                            entity_canonical_name: entity.canonical_name.clone(),
                            category: ObservationCategory::SuddenAnomaly,
                            title: format!("Sudden anomaly in action '{}'", ev.event_type),
                            description: format!(
                                "Entity {} performed action '{}' with high anomaly score ({:.2})",
                                entity.canonical_name, ev.event_type, scored.fused_score
                            ),
                            anomaly_score: scored.fused_score,
                            timestamp: ev.timestamp,
                        });
                    } else if scored.graph_score == 1.0 {
                        observations.push(BehaviorObservation {
                            entity_id: entity.id,
                            entity_canonical_name: entity.canonical_name.clone(),
                            category: ObservationCategory::NewBehavior,
                            title: format!("New interaction partner for {}", entity.canonical_name),
                            description: format!(
                                "Entity {} established a relationship with an unobserved target entity",
                                entity.canonical_name
                            ),
                            anomaly_score: scored.graph_score,
                            timestamp: ev.timestamp,
                        });
                    }
                }
            }
        }

        Ok(observations)
    }

    /// Identify and rank entities whose behavioral profiles have changed most significantly
    pub fn get_most_changed_entities(&self, limit: usize) -> Result<Vec<MostChangedEntity>> {
        let entities = EntityRepository::find_all(&self.storage)?;
        let mut changed_list = Vec::new();

        for entity in entities {
            if let Some(layers) = self.baseline_engine.load_temporal_layers(&entity.id)? {
                let drift = layers.calculate_drift();
                let anomaly = layers.calculate_anomaly();
                let combined = 0.5 * drift + 0.5 * anomaly;

                if combined > 0.0 {
                    changed_list.push(MostChangedEntity {
                        entity_id: entity.id,
                        canonical_name: entity.canonical_name,
                        entity_type: entity.entity_type,
                        drift_score: drift,
                        anomaly_score: anomaly,
                        combined_change_score: combined,
                        new_behaviors_count: if anomaly > 0.5 { 1 } else { 0 },
                    });
                }
            }
        }

        // Sort descending by combined_change_score
        changed_list.sort_by(|a, b| b.combined_change_score.partial_cmp(&a.combined_change_score).unwrap_or(std::cmp::Ordering::Equal));
        changed_list.truncate(limit);

        Ok(changed_list)
    }
}
