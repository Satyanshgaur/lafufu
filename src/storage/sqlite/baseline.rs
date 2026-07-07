use crate::behavior::Baseline;
use crate::errors::{LafufuError, Result};
use crate::repository::BaselineRepository;
use crate::storage::sqlite::SqliteStorage;
use rusqlite::params;
use uuid::Uuid;

impl BaselineRepository for SqliteStorage {
    fn save(&self, baseline: &Baseline) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let entity_id_str = baseline.entity_id.to_string();
        let profile_data_str = serde_json::to_string(&baseline.profile_data)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO baselines (entity_id, profile_type, schema_version, profile_data, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entity_id_str,
                baseline.profile_type,
                baseline.schema_version,
                profile_data_str,
                baseline.updated_at,
            ],
        )?;
        Ok(())
    }

    fn find_by_entity(&self, entity_id: &Uuid, profile_type: &str) -> Result<Option<Baseline>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let entity_id_str = entity_id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT entity_id, profile_type, schema_version, profile_data, updated_at 
             FROM baselines 
             WHERE entity_id = ?1 AND profile_type = ?2",
        )?;
        
        let mut rows = stmt.query(params![entity_id_str, profile_type])?;
        if let Some(row) = rows.next()? {
            let entity_id_parsed: Uuid = row.get::<_, String>(0)?.parse().map_err(|_| LafufuError::IdentityResolution("Invalid UUID".to_string()))?;
            let profile_data_str: String = row.get(3)?;
            let profile_data: serde_json::Value = serde_json::from_str(&profile_data_str)?;
            
            Ok(Some(Baseline {
                entity_id: entity_id_parsed,
                profile_type: row.get(1)?,
                schema_version: row.get(2)?,
                profile_data,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }
}
