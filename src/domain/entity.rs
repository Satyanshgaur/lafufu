use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Deterministic namespace for Lafufu Uuidv5 generation (DNS/URL/OID style but custom for Lafufu)
pub const LAFUFU_NAMESPACE: Uuid = Uuid::from_bytes([
    0x5c, 0xa6, 0x76, 0x76, 0x08, 0x05, 0x4c, 0x21, 
    0xb8, 0x8f, 0x3e, 0xc5, 0x64, 0x6c, 0x6e, 0xa8
]);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: String,
    pub canonical_name: String,
    pub attributes: serde_json::Value,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl Entity {
    pub fn new(entity_type: &str, canonical_name: &str, attributes: serde_json::Value, valid_from: DateTime<Utc>) -> Self {
        let id = Self::generate_id(entity_type, canonical_name);
        Self {
            id,
            entity_type: entity_type.to_string(),
            canonical_name: canonical_name.to_string(),
            attributes,
            valid_from,
            valid_to: None,
        }
    }

    /// Generate a deterministic UUIDv5 based on entity type and canonical name
    pub fn generate_id(entity_type: &str, canonical_name: &str) -> Uuid {
        let name_bytes = format!("{}:{}", entity_type, canonical_name);
        Uuid::new_v5(&LAFUFU_NAMESPACE, name_bytes.as_bytes())
    }
}
