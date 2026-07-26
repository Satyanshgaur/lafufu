use chrono::{Duration, Utc};
use lafufu::behavior::BaselineEngine;
use lafufu::detection::{DetectionEngine, FusionEngine, FusionWeights};
use lafufu::domain::{Entity, Event};
use lafufu::repository::{EntityRepository, EventRepository};
use lafufu::storage::sqlite::SqliteStorage;
use std::sync::Arc;
use uuid::Uuid;

fn setup_db() -> (SqliteStorage, Arc<BaselineEngine>, DetectionEngine) {
    let db_path = format!("{}/test_detection_{}.db", std::env::temp_dir().display(), Uuid::new_v4());
    let storage = SqliteStorage::new(&db_path).unwrap();
    let baseline_engine = Arc::new(BaselineEngine::with_default_windows(storage.clone()));
    let detection_engine = DetectionEngine::new(storage.clone(), baseline_engine.clone());
    (storage, baseline_engine, detection_engine)
}

#[test]
fn test_fusion_weight_calculations() {
    let user_weights = FusionWeights::for_entity_type("user");
    let service_weights = FusionWeights::for_entity_type("service");

    assert_eq!(user_weights.weight_sequence, 0.40);
    assert_eq!(service_weights.weight_velocity, 0.40);

    let fused = FusionEngine::fuse(1.0, 1.0, 1.0, &user_weights);
    assert_eq!(fused, 1.0);
}

#[test]
fn test_detection_engine_scoring() {
    let (storage, baseline_engine, detection_engine) = setup_db();
    let now = Utc::now();

    // 1. Create User Entity
    let user_entity = Entity::new("user", "alice", serde_json::json!({}), now - Duration::days(30));
    EntityRepository::save(&storage, &user_entity).unwrap();

    // 2. Populate historical baseline events
    for i in 0..10 {
        let ev = Event::new("ssh_login_success", now - Duration::days(i + 1), user_entity.id, None, serde_json::json!({"ip": "192.168.1.5"}));
        EventRepository::save(&storage, &ev).unwrap();
    }
    baseline_engine.compute_baseline_for_entity(&user_entity, now).unwrap();

    // 3. Score a normal event vs an anomalous new behavior event
    let normal_event = Event::new("ssh_login_success", now, user_entity.id, None, serde_json::json!({"ip": "192.168.1.5"}));
    let scored_normal = detection_engine.score_event(&normal_event, &user_entity).unwrap();

    let anomalous_event = Event::new("unauthorized_admin_escalation", now, user_entity.id, Some(Uuid::new_v4()), serde_json::json!({"ip": "10.99.99.99"}));
    let scored_anomalous = detection_engine.score_event(&anomalous_event, &user_entity).unwrap();

    assert!(scored_anomalous.fused_score > scored_normal.fused_score);
    assert_eq!(scored_anomalous.graph_score, 1.0); // Brand new target entity
}

#[test]
fn test_most_changed_entities_ranking() {
    let (storage, baseline_engine, detection_engine) = setup_db();
    let now = Utc::now();

    let user_a = Entity::new("user", "alice", serde_json::json!({}), now - Duration::days(30));
    let user_b = Entity::new("user", "bob", serde_json::json!({}), now - Duration::days(30));

    EntityRepository::save(&storage, &user_a).unwrap();
    EntityRepository::save(&storage, &user_b).unwrap();

    // Alice has consistent events across short and long term
    for i in 0..15 {
        let ev = Event::new("ssh_login", now - Duration::days(i), user_a.id, None, serde_json::json!({}));
        EventRepository::save(&storage, &ev).unwrap();
    }

    // Bob has sudden spike in short-term
    for _ in 0..20 {
        let ev = Event::new("failed_sudo", now - Duration::hours(2), user_b.id, None, serde_json::json!({}));
        EventRepository::save(&storage, &ev).unwrap();
    }

    baseline_engine.recompute_all_baselines(now).unwrap();

    let most_changed = detection_engine.get_most_changed_entities(5).unwrap();
    assert!(!most_changed.is_empty());
}
