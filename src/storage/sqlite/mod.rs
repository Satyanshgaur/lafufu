pub mod baseline;
pub mod edge;
pub mod entity;
pub mod event;

use crate::errors::{LafufuError, Result};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{info, debug};

#[derive(Clone)]
pub struct SqliteStorage {
    pub conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(db_path: &str) -> Result<Self> {
        info!("Initializing SQLite storage at: {}", db_path);
        let conn = Connection::open(db_path)?;
        
        // Optimize performance with WAL and synchronous settings
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;"
        )?;
        
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        
        storage.run_migrations()?;
        
        Ok(storage)
    }

    /// Run Phase 0 schema initialization and indexing
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| LafufuError::Analysis("Mutex poisoned".to_string()))?; // Mutex poison workaround
        
        debug!("Running initial schema migrations...");
        
        // 1. Entities table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                canonical_name TEXT NOT NULL,
                attributes TEXT NOT NULL,
                valid_from TEXT NOT NULL,
                valid_to TEXT
            );",
            [],
        )?;

        // 2. Events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                source_id TEXT NOT NULL,
                target_id TEXT,
                context TEXT NOT NULL
            );",
            [],
        )?;

        // 3. Edges table (Behavior graph relationships)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS edges (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                rel_type TEXT NOT NULL,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id, rel_type)
            );",
            [],
        )?;

        // 4. Baselines table (Derived profile baselines)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS baselines (
                entity_id TEXT NOT NULL,
                profile_type TEXT NOT NULL,
                schema_version TEXT NOT NULL,
                profile_data TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (entity_id, profile_type)
            );",
            [],
        )?;

        debug!("Creating database indexes...");
        
        // Create indexes as requested for fast timeline lookups and searches
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_source_id ON events(source_id);", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_target_id ON events(target_id);", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_source_target ON edges(source_id, target_id);", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_baselines_entity_id ON baselines(entity_id);", [])?;

        info!("Database schema migrations and indexing completed.");
        Ok(())
    }
}
