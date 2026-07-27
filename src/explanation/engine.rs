use crate::behavior::engine::BaselineEngine;
use crate::detection::engine::DetectionEngine;
use crate::explanation::ask::ConversationalQueryEngine;
use crate::explanation::diff::ProfileDiffEngine;
use crate::explanation::export::ExportEngine;
use crate::explanation::templates::ExplanationTemplates;
use crate::explanation::timeline::TimelineGenerator;
use crate::errors::{LafufuError, Result};
use crate::repository::EntityRepository;
use crate::storage::sqlite::SqliteStorage;
use chrono::{Duration, Utc};
use std::sync::Arc;

pub struct ExplanationEngine {
    storage: SqliteStorage,
    detection_engine: Arc<DetectionEngine>,
    baseline_engine: Arc<BaselineEngine>,
}

impl ExplanationEngine {
    pub fn new(
        storage: SqliteStorage,
        detection_engine: Arc<DetectionEngine>,
        baseline_engine: Arc<BaselineEngine>,
    ) -> Self {
        Self {
            storage,
            detection_engine,
            baseline_engine,
        }
    }

    /// Generate natural language briefing narrative for `lafufu explain --since <window>`
    pub fn generate_explain_report(&self, since_str: &str) -> Result<String> {
        let now = Utc::now();
        let since = if since_str.ends_with('h') {
            let hours: i64 = since_str.trim_end_matches('h').parse().unwrap_or(24);
            now - Duration::hours(hours)
        } else if since_str.ends_with('d') {
            let days: i64 = since_str.trim_end_matches('d').parse().unwrap_or(7);
            now - Duration::days(days)
        } else {
            now - Duration::hours(24)
        };

        let observations = self.detection_engine.detect_observations(since)?;
        let most_changed = self.detection_engine.get_most_changed_entities(5)?;

        Ok(ExplanationTemplates::format_daily_briefing(since_str, &observations, &most_changed))
    }

    /// Generate entity timeline narrative for `lafufu timeline <entity>`
    pub fn generate_timeline(&self, entity_name: &str) -> Result<String> {
        let entities = EntityRepository::find_all(&self.storage)?;
        let entity = entities
            .into_iter()
            .find(|e| e.canonical_name.eq_ignore_ascii_case(entity_name))
            .ok_or_else(|| LafufuError::Analysis(format!("Entity '{}' not found in storage", entity_name)))?;

        TimelineGenerator::build_timeline(&entity, &self.storage)
    }

    /// Generate profile diff for `lafufu diff <entity>`
    pub fn generate_diff(&self, entity_name: &str) -> Result<String> {
        let entities = EntityRepository::find_all(&self.storage)?;
        let entity = entities
            .into_iter()
            .find(|e| e.canonical_name.eq_ignore_ascii_case(entity_name))
            .ok_or_else(|| LafufuError::Analysis(format!("Entity '{}' not found in storage", entity_name)))?;

        ProfileDiffEngine::diff_entity_profiles(&entity, &self.baseline_engine)
    }

    /// Process natural language question for `lafufu ask "<query>"`
    pub fn process_ask_query(&self, query: &str) -> Result<String> {
        ConversationalQueryEngine::answer_question(query, &self.storage, &self.detection_engine)
    }

    /// Export table for `lafufu export <table>`
    pub fn export_table(&self, table: &str) -> Result<String> {
        ExportEngine::export_table_json(table, &self.storage)
    }
}
