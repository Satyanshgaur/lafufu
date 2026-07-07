use crate::domain::{Edge, Entity, Event};
use crate::behavior::Baseline;
use crate::errors::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub trait EntityRepository: Send + Sync {
    fn save(&self, entity: &Entity) -> Result<()>;
    fn find_by_id(&self, id: &Uuid) -> Result<Option<Entity>>;
    fn find_version_at(&self, id: &Uuid, timestamp: DateTime<Utc>) -> Result<Option<Entity>>;
    fn find_all(&self) -> Result<Vec<Entity>>;
}

pub trait EventRepository: Send + Sync {
    fn save(&self, event: &Event) -> Result<()>;
    fn find_by_id(&self, id: &Uuid) -> Result<Option<Event>>;
    fn find_by_entity(&self, entity_id: &Uuid, since: DateTime<Utc>) -> Result<Vec<Event>>;
    fn find_all_since(&self, since: DateTime<Utc>) -> Result<Vec<Event>>;
}

pub trait EdgeRepository: Send + Sync {
    fn save(&self, edge: &Edge) -> Result<()>;
    fn find_edges_for_entity(&self, entity_id: &Uuid) -> Result<Vec<Edge>>;
    fn find_all(&self) -> Result<Vec<Edge>>;
}

pub trait BaselineRepository: Send + Sync {
    fn save(&self, baseline: &Baseline) -> Result<()>;
    fn find_by_entity(&self, entity_id: &Uuid, profile_type: &str) -> Result<Option<Baseline>>;
}
