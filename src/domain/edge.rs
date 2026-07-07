use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub rel_type: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl Edge {
    pub fn new(source_id: Uuid, target_id: Uuid, rel_type: &str, timestamp: DateTime<Utc>) -> Self {
        Self {
            source_id,
            target_id,
            rel_type: rel_type.to_string(),
            first_seen: timestamp,
            last_seen: timestamp,
        }
    }
}
