use crate::errors::{LafufuError, Result};
use crate::repository::{EdgeRepository, EntityRepository, EventRepository};
use crate::storage::sqlite::SqliteStorage;
use chrono::Utc;

pub struct ExportEngine;

impl ExportEngine {
    /// Export database table contents to JSON format
    pub fn export_table_json(table: &str, storage: &SqliteStorage) -> Result<String> {
        match table.to_lowercase().as_str() {
            "entities" | "entity" => {
                let entities = EntityRepository::find_all(storage)?;
                Ok(serde_json::to_string_pretty(&entities)?)
            }
            "events" | "event" => {
                let events = EventRepository::find_all_since(storage, chrono::DateTime::<Utc>::MIN_UTC)?;
                Ok(serde_json::to_string_pretty(&events)?)
            }
            "edges" | "edge" => {
                let edges = EdgeRepository::find_all(storage)?;
                Ok(serde_json::to_string_pretty(&edges)?)
            }
            _ => Err(LafufuError::Ingestion(format!("Invalid export table target: '{}'. Valid options: entities, events, edges", table))),
        }
    }
}
