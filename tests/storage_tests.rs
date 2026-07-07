use lafufu::domain::{Edge, Entity, Event};
use lafufu::behavior::Baseline;
use lafufu::repository::{BaselineRepository, EdgeRepository, EntityRepository, EventRepository};
use lafufu::storage::sqlite::SqliteStorage;
use chrono::Utc;
use serde_json::json;

#[test]
fn test_sqlite_repositories() {
    // Open a temporary database in memory
    let storage = SqliteStorage::new(":memory:").expect("Failed to initialize memory SQLite");

    // 1. Test Entity Repository
    let now = Utc::now();
    let entity = Entity::new(
        "user",
        "alice",
        json!({"department": "engineering", "role": "admin"}),
        now,
    );
    
    // Save entity using fully qualified trait method
    EntityRepository::save(&storage, &entity).expect("Failed to save entity");
    
    // Retrieve entity
    let retrieved = EntityRepository::find_by_id(&storage, &entity.id)
        .expect("Failed to fetch entity")
        .expect("Entity not found");
        
    assert_eq!(retrieved.entity_type, "user");
    assert_eq!(retrieved.canonical_name, "alice");
    assert_eq!(retrieved.attributes["department"], "engineering");

    // 2. Test Event Repository
    let event = Event::new(
        "auth.login",
        now,
        entity.id,
        None,
        json!({"ip": "127.0.0.1", "status": "success"}),
    );
    
    // Save event
    EventRepository::save(&storage, &event).expect("Failed to save event");
    
    // Retrieve event
    let retrieved_event = EventRepository::find_by_id(&storage, &event.id)
        .expect("Failed to fetch event")
        .expect("Event not found");
        
    assert_eq!(retrieved_event.event_type, "auth.login");
    assert_eq!(retrieved_event.source_id, entity.id);
    assert_eq!(retrieved_event.context["ip"], "127.0.0.1");

    // 3. Test Edge Repository
    let service_entity = Entity::new(
        "service",
        "checkout",
        json!({"version": "v1.2"}),
        now,
    );
    EntityRepository::save(&storage, &service_entity).expect("Failed to save service entity");

    let edge = Edge::new(entity.id, service_entity.id, "communicated_with", now);
    EdgeRepository::save(&storage, &edge).expect("Failed to save edge");

    let retrieved_edges = EdgeRepository::find_edges_for_entity(&storage, &entity.id).expect("Failed to fetch edges");
    assert_eq!(retrieved_edges.len(), 1);
    assert_eq!(retrieved_edges[0].source_id, entity.id);
    assert_eq!(retrieved_edges[0].target_id, service_entity.id);
    assert_eq!(retrieved_edges[0].rel_type, "communicated_with");

    // 4. Test Baseline Repository
    let baseline = Baseline::new(
        entity.id,
        "user",
        "V1",
        json!({"typical_hours": [9, 10, 11, 17]}),
    );
    BaselineRepository::save(&storage, &baseline).expect("Failed to save baseline");

    let retrieved_baseline = BaselineRepository::find_by_entity(&storage, &entity.id, "user")
        .expect("Failed to fetch baseline")
        .expect("Baseline not found");

    assert_eq!(retrieved_baseline.schema_version, "V1");
    assert_eq!(retrieved_baseline.profile_data["typical_hours"][0], 9);
}
