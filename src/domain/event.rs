use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub source_id: Uuid,
    pub target_id: Option<Uuid>,
    pub context: serde_json::Value,
}

impl Event {
    pub fn new(
        event_type: &str,
        timestamp: DateTime<Utc>,
        source_id: Uuid,
        target_id: Option<Uuid>,
        context: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(), // Events can be generated with a random UUIDv4 as they are distinct occurrences
            event_type: event_type.to_string(),
            timestamp,
            source_id,
            target_id,
            context,
        }
    }
}
