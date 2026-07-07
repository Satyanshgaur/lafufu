use crate::domain::Entity;
use crate::errors::{LafufuError, Result};
use crate::repository::EntityRepository;
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

impl EntityRepository for SqliteStorage {
    fn save(&self, entity: &Entity) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let attributes_str = serde_json::to_string(&entity.attributes)?;
        let id_str = entity.id.to_string();
        
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, entity_type, canonical_name, attributes, valid_from, valid_to)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id_str,
                entity.entity_type,
                entity.canonical_name,
                attributes_str,
                entity.valid_from,
                entity.valid_to,
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &Uuid) -> Result<Option<Entity>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let id_str = id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, canonical_name, attributes, valid_from, valid_to 
             FROM entities WHERE id = ?1 AND valid_to IS NULL",
        )?;
        
        let mut rows = stmt.query(params![id_str])?;
        if let Some(row) = rows.next()? {
            let id_parsed: Uuid = row.get::<_, String>(0)?.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?;
            let attributes_str: String = row.get(3)?;
            let attributes: serde_json::Value = serde_json::from_str(&attributes_str)?;
            
            Ok(Some(Entity {
                id: id_parsed,
                entity_type: row.get(1)?,
                canonical_name: row.get(2)?,
                attributes,
                valid_from: row.get(4)?,
                valid_to: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn find_version_at(&self, id: &Uuid, timestamp: DateTime<Utc>) -> Result<Option<Entity>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let id_str = id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, canonical_name, attributes, valid_from, valid_to 
             FROM entities 
             WHERE id = ?1 AND valid_from <= ?2 AND (valid_to IS NULL OR valid_to > ?2)",
        )?;
        
        let mut rows = stmt.query(params![id_str, timestamp])?;
        if let Some(row) = rows.next()? {
            let id_parsed: Uuid = row.get::<_, String>(0)?.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?;
            let attributes_str: String = row.get(3)?;
            let attributes: serde_json::Value = serde_json::from_str(&attributes_str)?;
            
            Ok(Some(Entity {
                id: id_parsed,
                entity_type: row.get(1)?,
                canonical_name: row.get(2)?,
                attributes,
                valid_from: row.get(4)?,
                valid_to: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn find_all(&self) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, canonical_name, attributes, valid_from, valid_to 
             FROM entities WHERE valid_to IS NULL",
        )?;
        
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let attributes_str: String = row.get(3)?;
            Ok((id_str, row.get::<_, String>(1)?, row.get::<_, String>(2)?, attributes_str, row.get::<_, DateTime<Utc>>(4)?, row.get::<_, Option<DateTime<Utc>>>(5)?))
        })?;

        let mut entities = Vec::new();
        for row_res in rows {
            let (id_str, entity_type, canonical_name, attr_str, valid_from, valid_to) = row_res?;
            let id_parsed = id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let attributes = serde_json::from_str(&attr_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
            entities.push(Entity {
                id: id_parsed,
                entity_type,
                canonical_name,
                attributes,
                valid_from,
                valid_to,
            });
        }
        Ok(entities)
    }
}
