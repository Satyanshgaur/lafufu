use crate::domain::Edge;
use crate::errors::{LafufuError, Result};
use crate::repository::EdgeRepository;
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

impl EdgeRepository for SqliteStorage {
    fn save(&self, edge: &Edge) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let source_str = edge.source_id.to_string();
        let target_str = edge.target_id.to_string();
        
        conn.execute(
            "INSERT INTO edges (source_id, target_id, rel_type, first_seen, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(source_id, target_id, rel_type) DO UPDATE SET
             last_seen = excluded.last_seen",
            params![
                source_str,
                target_str,
                edge.rel_type,
                edge.first_seen,
                edge.last_seen,
            ],
        )?;
        Ok(())
    }

    fn find_edges_for_entity(&self, entity_id: &Uuid) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        let entity_str = entity_id.to_string();
        
        let mut stmt = conn.prepare(
            "SELECT source_id, target_id, rel_type, first_seen, last_seen 
             FROM edges 
             WHERE source_id = ?1 OR target_id = ?1",
        )?;
        
        let rows = stmt.query_map(params![entity_str], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, DateTime<Utc>>(3)?,
                row.get::<_, DateTime<Utc>>(4)?,
            ))
        })?;

        let mut edges = Vec::new();
        for row_res in rows {
            let (source_id_str, target_id_str, rel_type, first_seen, last_seen) = row_res?;
            let source_id = source_id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let target_id = target_id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            edges.push(Edge {
                source_id,
                target_id,
                rel_type,
                first_seen,
                last_seen,
            });
        }
        Ok(edges)
    }

    fn find_all(&self) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?;
        
        let mut stmt = conn.prepare(
            "SELECT source_id, target_id, rel_type, first_seen, last_seen FROM edges",
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, DateTime<Utc>>(3)?,
                row.get::<_, DateTime<Utc>>(4)?,
            ))
        })?;

        let mut edges = Vec::new();
        for row_res in rows {
            let (source_id_str, target_id_str, rel_type, first_seen, last_seen) = row_res?;
            let source_id = source_id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let target_id = target_id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            edges.push(Edge {
                source_id,
                target_id,
                rel_type,
                first_seen,
                last_seen,
            });
        }
        Ok(edges)
    }
}
