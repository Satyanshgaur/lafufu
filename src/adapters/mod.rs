pub mod auth;
pub mod docker;
pub mod github;
pub mod json;

use crate::domain::{Edge, Entity, Event};
use crate::errors::Result;
use crate::normalization::IdentityResolver;

/// The normalized product of an adapter parsing raw log input
#[derive(Debug, Clone, Default)]
pub struct IngestionBatch {
    pub entities: Vec<Entity>,
    pub events: Vec<Event>,
    pub edges: Vec<Edge>,
}

impl IngestionBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.events.is_empty() && self.edges.is_empty()
    }

    pub fn merge(&mut self, mut other: IngestionBatch) {
        self.entities.append(&mut other.entities);
        self.events.append(&mut other.events);
        self.edges.append(&mut other.edges);
    }
}

/// The universal adapter interface for log parsing and behavior graph mapping
pub trait LogAdapter: Send + Sync {
    /// Human-readable identifier for the adapter
    fn name(&self) -> &str;

    /// Quick sample inspector to determine if this adapter can handle given content
    fn can_parse(&self, sample: &str) -> bool;

    /// Parse raw text content into behavior graph primitives
    fn parse(&self, content: &str, resolver: &IdentityResolver) -> Result<IngestionBatch>;
}
