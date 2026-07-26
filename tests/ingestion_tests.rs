use lafufu::ingestion::IngestionPipeline;
use lafufu::normalization::{IdentityConfig, IdentityResolver};
use lafufu::repository::{EdgeRepository, EntityRepository, EventRepository};
use lafufu::storage::sqlite::SqliteStorage;
use std::sync::Arc;
use uuid::Uuid;

fn setup_pipeline() -> (IngestionPipeline, SqliteStorage) {
    let db_path = format!("{}/test_{}.db", std::env::temp_dir().display(), Uuid::new_v4());
    let storage = SqliteStorage::new(&db_path).unwrap();
    let resolver = Arc::new(IdentityResolver::new(IdentityConfig::default()));
    let pipeline = IngestionPipeline::new(storage.clone(), resolver);
    (pipeline, storage)
}

#[test]
fn test_json_log_ingestion() {
    let (pipeline, storage) = setup_pipeline();
    let json_data = r#"{"timestamp": "2026-07-26T18:00:00Z", "user": "alice", "action": "login", "target": "auth_server"}
{"timestamp": "2026-07-26T18:05:00Z", "user": "bob", "action": "file_access", "target": "secret.txt"}"#;

    let report = pipeline.process_str(json_data, Some("generic_json")).unwrap();

    assert_eq!(report.events_ingested, 2);
    assert_eq!(report.entities_created, 4); // alice, auth_server, bob, secret.txt
    assert_eq!(report.edges_updated, 2);

    let events = EventRepository::find_all_since(&storage, chrono::Utc::now() - chrono::Duration::hours(24)).unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn test_syslog_auth_ingestion() {
    let (pipeline, storage) = setup_pipeline();
    let auth_log = r#"Jul 26 18:00:00 server sshd[1234]: Accepted password for alice from 192.168.1.50 port 54321 ssh2
Jul 26 18:02:00 server sudo: alice : TTY=pts/0 ; PWD=/home/alice ; USER=root ; COMMAND=/bin/cat /etc/shadow"#;

    let report = pipeline.process_str(auth_log, Some("syslog_auth")).unwrap();

    assert_eq!(report.events_ingested, 2);
    let entities = EntityRepository::find_all(&storage).unwrap();
    assert!(!entities.is_empty());
}

#[test]
fn test_github_events_ingestion() {
    let (pipeline, storage) = setup_pipeline();
    let github_log = r#"{
        "type": "PushEvent",
        "created_at": "2026-07-26T18:00:00Z",
        "actor": {"login": "octocat"},
        "repo": {"name": "octocat/Hello-World"},
        "payload": {"commits": [{"sha": "12345"}]}
    }"#;

    let report = pipeline.process_str(github_log, Some("github_events")).unwrap();

    assert_eq!(report.events_ingested, 1);
    let edges = EdgeRepository::find_all(&storage).unwrap();
    assert_eq!(edges[0].rel_type, "pushed_to");
}

#[test]
fn test_docker_events_ingestion() {
    let (pipeline, storage) = setup_pipeline();
    let docker_log = r#"{
        "status": "start",
        "id": "c123456789",
        "from": "nginx:latest",
        "Type": "container",
        "action": "start",
        "Actor": {
            "ID": "c123456789",
            "Attributes": {
                "image": "nginx:latest",
                "name": "web_server"
            }
        },
        "time": 1719829200
    }"#;

    let report = pipeline.process_str(docker_log, Some("docker_events")).unwrap();

    assert_eq!(report.events_ingested, 1);
    assert_eq!(report.adapter_used, "docker_events");

    let events = EventRepository::find_all_since(&storage, chrono::DateTime::<chrono::Utc>::MIN_UTC).unwrap();
    assert_eq!(events.len(), 1);
}
