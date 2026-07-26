use crate::adapters::{
    auth::SyslogAuthAdapter, docker::DockerEventsAdapter, github::GitHubEventsAdapter, json::JsonLogAdapter,
    LogAdapter,
};
use crate::errors::{LafufuError, Result};
use crate::normalization::IdentityResolver;
use crate::repository::{EdgeRepository, EntityRepository, EventRepository};
use crate::storage::sqlite::SqliteStorage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestionReport {
    pub events_ingested: usize,
    pub entities_created: usize,
    pub edges_updated: usize,
    pub adapter_used: String,
}

pub struct IngestionPipeline {
    adapters: Vec<Box<dyn LogAdapter>>,
    resolver: Arc<IdentityResolver>,
    storage: SqliteStorage,
}

impl IngestionPipeline {
    pub fn new(storage: SqliteStorage, resolver: Arc<IdentityResolver>) -> Self {
        let adapters: Vec<Box<dyn LogAdapter>> = vec![
            Box::new(JsonLogAdapter::new()),
            Box::new(SyslogAuthAdapter::new()),
            Box::new(GitHubEventsAdapter::new()),
            Box::new(DockerEventsAdapter::new()),
        ];

        Self {
            adapters,
            resolver,
            storage,
        }
    }

    /// Register a custom log adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn LogAdapter>) {
        self.adapters.push(adapter);
    }

    /// Select the best matching adapter based on sample lines
    pub fn select_adapter<'a>(&'a self, sample: &str, preferred_adapter: Option<&str>) -> Result<&'a dyn LogAdapter> {
        if let Some(pref) = preferred_adapter {
            for adapter in &self.adapters {
                if adapter.name().eq_ignore_ascii_case(pref) {
                    return Ok(adapter.as_ref());
                }
            }
            return Err(LafufuError::Ingestion(format!("Adapter '{}' not found", pref)));
        }

        // Auto-detect based on can_parse
        for adapter in &self.adapters {
            if adapter.can_parse(sample) {
                return Ok(adapter.as_ref());
            }
        }

        // Fall back to generic JSON or error
        if self.adapters[0].can_parse(sample) {
            Ok(self.adapters[0].as_ref())
        } else {
            // Default to first adapter if non-empty
            Ok(self.adapters[0].as_ref())
        }
    }

    /// Process a raw log string and save all entities, events, and edges
    pub fn process_str(&self, raw_content: &str, preferred_adapter: Option<&str>) -> Result<IngestionReport> {
        if raw_content.trim().is_empty() {
            return Ok(IngestionReport::default());
        }

        let adapter = self.select_adapter(raw_content, preferred_adapter)?;
        info!("Using log adapter '{}' for ingestion", adapter.name());

        let batch = adapter.parse(raw_content, &self.resolver)?;

        let mut entities_count = 0;
        let mut events_count = 0;
        let mut edges_count = 0;

        // 1. Save Entities
        for entity in &batch.entities {
            if EntityRepository::find_by_id(&self.storage, &entity.id)?.is_none() {
                EntityRepository::save(&self.storage, entity)?;
                entities_count += 1;
            } else {
                // Update entity valid attributes
                EntityRepository::save(&self.storage, entity)?;
            }
        }

        // 2. Save Events
        for event in &batch.events {
            EventRepository::save(&self.storage, event)?;
            events_count += 1;
        }

        // 3. Save Edges (Upsert with updated last_seen)
        for edge in &batch.edges {
            EdgeRepository::save(&self.storage, edge)?;
            edges_count += 1;
        }

        info!(
            "Ingestion complete: {} events, {} entities, {} edges",
            events_count, entities_count, edges_count
        );

        Ok(IngestionReport {
            events_ingested: events_count,
            entities_created: entities_count,
            edges_updated: edges_count,
            adapter_used: adapter.name().to_string(),
        })
    }

    /// Process a log file from path
    pub fn process_file<P: AsRef<Path>>(&self, path: P, preferred_adapter: Option<&str>) -> Result<IngestionReport> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(LafufuError::Ingestion(format!("File not found: {:?}", p)));
        }

        let content = fs::read_to_string(p)?;
        self.process_str(&content, preferred_adapter)
    }

    /// Process an entire directory of log files recursively
    pub fn process_directory<P: AsRef<Path>>(&self, dir: P, preferred_adapter: Option<&str>) -> Result<IngestionReport> {
        let p = dir.as_ref();
        if !p.is_dir() {
            return Err(LafufuError::Ingestion(format!("Not a directory: {:?}", p)));
        }

        let mut aggregate = IngestionReport::default();

        for entry in fs::read_dir(p)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                match self.process_file(&path, preferred_adapter) {
                    Ok(rep) => {
                        aggregate.events_ingested += rep.events_ingested;
                        aggregate.entities_created += rep.entities_created;
                        aggregate.edges_updated += rep.edges_updated;
                        aggregate.adapter_used = rep.adapter_used;
                    }
                    Err(e) => {
                        warn!("Skipping file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(aggregate)
    }
}
