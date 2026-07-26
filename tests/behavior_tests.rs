use chrono::{Duration, Utc};
use lafufu::behavior::{BaselineEngine, CategoricalHistogram, Profile, RunningStats};
use lafufu::domain::{Entity, Event};
use lafufu::repository::{BaselineRepository, EntityRepository, EventRepository};
use lafufu::storage::sqlite::SqliteStorage;
use uuid::Uuid;

fn setup_db() -> SqliteStorage {
    let db_path = format!("{}/test_behavior_{}.db", std::env::temp_dir().display(), Uuid::new_v4());
    SqliteStorage::new(&db_path).unwrap()
}

#[test]
fn test_running_stats_math() {
    let mut stats = RunningStats::new();
    stats.update(100.0);
    stats.update(200.0);
    stats.update(300.0);

    assert_eq!(stats.mean, 200.0);
    assert!((stats.std_dev() - 100.0).abs() < 1e-5);
    assert_eq!(stats.z_score(400.0), 2.0);
}

#[test]
fn test_histogram_and_jsd() {
    let mut h1 = CategoricalHistogram::new();
    h1.observe("ssh_login_success");
    h1.observe("ssh_login_success");
    h1.observe("ssh_login_success");

    let mut h2 = CategoricalHistogram::new();
    h2.observe("ssh_login_success");
    h2.observe("sudo_exec");
    h2.observe("sudo_exec");

    let jsd = h1.jensen_shannon_divergence(&h2);
    assert!(jsd > 0.0);

    let self_jsd = h1.jensen_shannon_divergence(&h1);
    assert!(self_jsd < 1e-5);
}

#[test]
fn test_baseline_engine_temporal_computation() {
    let storage = setup_db();
    let engine = BaselineEngine::with_default_windows(storage.clone());

    let now = Utc::now();

    // 1. Create a User Entity
    let user_entity = Entity::new("user", "alice", serde_json::json!({}), now - Duration::days(100));
    EntityRepository::save(&storage, &user_entity).unwrap();

    // 2. Add historical events across short, medium, long term
    let event_long = Event::new("ssh_login_success", now - Duration::days(40), user_entity.id, None, serde_json::json!({"ip": "192.168.1.10"}));
    let event_medium = Event::new("ssh_login_success", now - Duration::days(10), user_entity.id, None, serde_json::json!({"ip": "192.168.1.10"}));
    let event_short = Event::new("sudo_exec", now - Duration::hours(12), user_entity.id, None, serde_json::json!({"command": "/usr/bin/apt update"}));

    EventRepository::save(&storage, &event_long).unwrap();
    EventRepository::save(&storage, &event_medium).unwrap();
    EventRepository::save(&storage, &event_short).unwrap();

    // 3. Compute baselines
    let layers = engine.compute_baseline_for_entity(&user_entity, now).unwrap();

    // Verify profiles exist across temporal horizons
    assert!(matches!(layers.short_term, Profile::User(_)));
    assert!(matches!(layers.medium_term, Profile::User(_)));
    assert!(matches!(layers.long_term, Profile::User(_)));

    // Verify persistence in SQLite baselines table
    let saved_short = BaselineRepository::find_by_entity(&storage, &user_entity.id, "short_term").unwrap();
    assert!(saved_short.is_some());
    assert_eq!(saved_short.unwrap().entity_id, user_entity.id);

    // Verify reloading
    let reloaded = engine.load_temporal_layers(&user_entity.id).unwrap();
    assert!(reloaded.is_some());
}
