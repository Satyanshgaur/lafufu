use crate::domain::Event;
use crate::errors::{LafufuError, Result};
use crate::repository::EventRepository;
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

impl EventRepository for SqliteStorage {
    fn save(&self, event: &Event) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let id_str = event.id.to_string();
        let source_id_str = event.source_id.to_string();
        let target_id_str = event.target_id.map(|id| id.to_string());
        let context_str = serde_json::to_string(&event.context)?;
        
        conn.execute(
            "INSERT INTO events (id, event_type, timestamp, source_id, target_id, context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id_str,
                event.event_type,
                event.timestamp,
                source_id_str,
                target_id_str,
                context_str,
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &Uuid) -> Result<Option<Event>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let id_str = id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT id, event_type, timestamp, source_id, target_id, context 
             FROM events WHERE id = ?1",
        )?;
        
        let mut rows = stmt.query(params![id_str])?;
        if let Some(row) = rows.next()? {
            let id_parsed: Uuid = row.get::<_, String>(0)?.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?;
            let source_parsed: Uuid = row.get::<_, String>(3)?.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?;
            let target_str: Option<String> = row.get(4)?;
            let target_parsed = match target_str {
                Some(t) => Some(t.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?),
                None => None,
            };
            let context_str: String = row.get(5)?;
            let context: serde_json::Value = serde_json::from_str(&context_str)?;
            
            Ok(Some(Event {
                id: id_parsed,
                event_type: row.get(1)?,
                timestamp: row.get(2)?,
                source_id: source_parsed,
                target_id: target_parsed,
                context,
            }))
        } else {
            Ok(None)
        }
    }

    fn find_by_entity(&self, entity_id: &Uuid, since: DateTime<Utc>) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let entity_id_str = entity_id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT id, event_type, timestamp, source_id, target_id, context 
             FROM events 
             WHERE (source_id = ?1 OR target_id = ?1) AND timestamp >= ?2
             ORDER BY timestamp ASC",
        )?;
        
        let rows = stmt.query_map(params![entity_id_str, since], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, DateTime<Utc>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut events = Vec::new();
        for row_res in rows {
            let (id_str, event_type, timestamp, source_str, target_str, context_str) = row_res?;
            let id_parsed = id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let source_parsed = source_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let target_parsed = match target_str {
                Some(t) => Some(t.parse().map_err(|_| rusqlite::Error::InvalidQuery)?),
                None => None,
            };
            let context = serde_json::from_str(&context_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
            events.push(Event {
                id: id_parsed,
                event_type,
                timestamp,
                source_id: source_parsed,
                target_id: target_parsed,
                context,
            });
        }
        Ok(events)
    }

    fn find_all_since(&self, since: DateTime<Utc>) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, event_type, timestamp, source_id, target_id, context 
             FROM events 
             WHERE timestamp >= ?1
             ORDER BY timestamp ASC",
        )?;
        
        let rows = stmt.query_map(params![since], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, DateTime<Utc>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut events = Vec::new();
        for row_res in rows {
            let (id_str, event_type, timestamp, source_str, target_str, context_str) = row_res?;
            let id_parsed = id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let source_parsed = source_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let target_parsed = match target_str {
                Some(t) => Some(t.parse().map_err(|_| rusqlite::Error::InvalidQuery)?),
                None => None,
            };
            let context = serde_json::from_str(&context_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
            events.push(Event {
                id: id_parsed,
                event_type,
                timestamp,
                source_id: source_parsed,
                target_id: target_parsed,
                context,
            });
        }
        Ok(events)
    }
}
