use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Baseline {
    pub entity_id: Uuid,
    pub profile_type: String,
    pub schema_version: String,
    pub profile_data: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

impl Baseline {
    pub fn new(entity_id: Uuid, profile_type: &str, schema_version: &str, profile_data: serde_json::Value) -> Self {
        Self {
            entity_id,
            profile_type: profile_type.to_string(),
            schema_version: schema_version.to_string(),
            profile_data,
            updated_at: Utc::now(),
        }
    }
}
