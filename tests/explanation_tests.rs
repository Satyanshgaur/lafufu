use chrono::{Duration, Utc};
use lafufu::behavior::BaselineEngine;
use lafufu::detection::DetectionEngine;
use lafufu::domain::{Entity, Event};
use lafufu::explanation::ExplanationEngine;
use lafufu::repository::{EntityRepository, EventRepository};
use lafufu::storage::sqlite::SqliteStorage;
use std::sync::Arc;
use uuid::Uuid;

fn setup_db() -> (SqliteStorage, ExplanationEngine) {
    let db_path = format!("{}/test_explanation_{}.db", std::env::temp_dir().display(), Uuid::new_v4());
    let storage = SqliteStorage::new(&db_path).unwrap();
    let baseline_engine = Arc::new(BaselineEngine::with_default_windows(storage.clone()));
    let detection_engine = Arc::new(DetectionEngine::new(storage.clone(), baseline_engine.clone()));
    let explanation_engine = ExplanationEngine::new(storage.clone(), detection_engine, baseline_engine);
    (storage, explanation_engine)
}

#[test]
fn test_explain_report_generation() {
    let (storage, explanation) = setup_db();
    let now = Utc::now();

    let user_entity = Entity::new("user", "alice", serde_json::json!({}), now - Duration::days(30));
    EntityRepository::save(&storage, &user_entity).unwrap();

    let ev = Event::new("ssh_login_success", now - Duration::hours(1), user_entity.id, None, serde_json::json!({}));
    EventRepository::save(&storage, &ev).unwrap();

    let briefing = explanation.generate_explain_report("24h").unwrap();
    assert!(briefing.contains("Lafufu Behavioral Intelligence Briefing"));
}

#[test]
fn test_timeline_generation() {
    let (storage, explanation) = setup_db();
    let now = Utc::now();

    let user_entity = Entity::new("user", "bob", serde_json::json!({}), now - Duration::days(10));
    EntityRepository::save(&storage, &user_entity).unwrap();

    let ev1 = Event::new("login", now - Duration::days(2), user_entity.id, None, serde_json::json!({}));
    let ev2 = Event::new("sudo_exec", now - Duration::days(1), user_entity.id, None, serde_json::json!({}));
    EventRepository::save(&storage, &ev1).unwrap();
    EventRepository::save(&storage, &ev2).unwrap();

    let timeline = explanation.generate_timeline("bob").unwrap();
    assert!(timeline.contains("Behavioral Timeline: bob"));
    assert!(timeline.contains("Phase 1"));
}

#[test]
fn test_conversational_ask_query() {
    let (storage, explanation) = setup_db();
    let now = Utc::now();

    let user_entity = Entity::new("service", "payment-api", serde_json::json!({}), now - Duration::days(5));
    EntityRepository::save(&storage, &user_entity).unwrap();

    let ans = explanation.process_ask_query("What changed today?").unwrap();
    assert!(ans.contains("Lafufu Conversational Query Response"));
}

#[test]
fn test_export_table_json() {
    let (storage, explanation) = setup_db();
    let now = Utc::now();

    let user_entity = Entity::new("user", "charlie", serde_json::json!({}), now);
    EntityRepository::save(&storage, &user_entity).unwrap();

    let json_export = explanation.export_table("entities").unwrap();
    assert!(json_export.contains("charlie"));
}
