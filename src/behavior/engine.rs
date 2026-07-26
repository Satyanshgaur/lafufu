use crate::behavior::baseline::Baseline;
use crate::behavior::profile::Profile;
use crate::behavior::temporal::{TemporalLayers, TemporalWindows};
use crate::domain::Entity;
use crate::errors::Result;
use crate::repository::{BaselineRepository, EdgeRepository, EntityRepository, EventRepository};
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Utc};
use tracing::{debug, info};
use uuid::Uuid;

pub struct BaselineEngine {
    storage: SqliteStorage,
    windows: TemporalWindows,
}

impl BaselineEngine {
    pub fn new(storage: SqliteStorage, windows: TemporalWindows) -> Self {
        Self { storage, windows }
    }

    pub fn with_default_windows(storage: SqliteStorage) -> Self {
        Self::new(storage, TemporalWindows::default())
    }

    /// Compute multi-layer temporal profiles for a given entity and save to baseline storage
    pub fn compute_baseline_for_entity(&self, entity: &Entity, now: DateTime<Utc>) -> Result<TemporalLayers> {
        let entity_id = entity.id;
        let entity_type = &entity.entity_type;

        debug!("Computing behavioral baselines for entity {} ({})", entity.canonical_name, entity_type);

        let long_since = self.windows.long_term_since(now);
        let medium_since = self.windows.medium_term_since(now);
        let short_since = self.windows.short_term_since(now);

        // 1. Query events and edges for entity
        let events = EventRepository::find_by_entity(&self.storage, &entity_id, long_since)?;
        let edges = EdgeRepository::find_edges_for_entity(&self.storage, &entity_id)?;

        // 2. Initialize 3 temporal profiles
        let mut short_profile = Profile::for_entity_type(entity_type);
        let mut medium_profile = Profile::for_entity_type(entity_type);
        let mut long_profile = Profile::for_entity_type(entity_type);

        // 3. Populate Event observations across time horizons
        for event in &events {
            let is_source = event.source_id == entity_id;
            long_profile.observe_event(event, is_source);

            if event.timestamp >= medium_since {
                medium_profile.observe_event(event, is_source);
            }
            if event.timestamp >= short_since {
                short_profile.observe_event(event, is_source);
            }
        }

        // 4. Populate Edge observations across time horizons
        for edge in &edges {
            let is_source = edge.source_id == entity_id;
            long_profile.observe_edge(edge, is_source);

            if edge.last_seen >= medium_since {
                medium_profile.observe_edge(edge, is_source);
            }
            if edge.last_seen >= short_since {
                short_profile.observe_edge(edge, is_source);
            }
        }

        // 5. Persist derived profiles to baselines storage table
        self.save_baseline_profile(&entity_id, "short_term", &short_profile)?;
        self.save_baseline_profile(&entity_id, "medium_term", &medium_profile)?;
        self.save_baseline_profile(&entity_id, "long_term", &long_profile)?;

        Ok(TemporalLayers::new(short_profile, medium_profile, long_profile))
    }

    /// Recompute baselines for all active entities in storage
    pub fn recompute_all_baselines(&self, now: DateTime<Utc>) -> Result<usize> {
        let entities = EntityRepository::find_all(&self.storage)?;
        info!("Recomputing behavioral baselines for {} entities...", entities.len());

        let mut count = 0;
        for entity in &entities {
            self.compute_baseline_for_entity(entity, now)?;
            count += 1;
        }

        info!("Successfully updated {} entity baseline profiles", count);
        Ok(count)
    }

    fn save_baseline_profile(&self, entity_id: &Uuid, profile_type: &str, profile: &Profile) -> Result<()> {
        let profile_data = serde_json::to_value(profile)?;
        let baseline = Baseline::new(*entity_id, profile_type, "v1", profile_data);
        BaselineRepository::save(&self.storage, &baseline)?;
        Ok(())
    }

    /// Load saved temporal layers for an entity from baseline storage
    pub fn load_temporal_layers(&self, entity_id: &Uuid) -> Result<Option<TemporalLayers>> {
        let short_b = BaselineRepository::find_by_entity(&self.storage, entity_id, "short_term")?;
        let medium_b = BaselineRepository::find_by_entity(&self.storage, entity_id, "medium_term")?;
        let long_b = BaselineRepository::find_by_entity(&self.storage, entity_id, "long_term")?;

        match (short_b, medium_b, long_b) {
            (Some(s), Some(m), Some(l)) => {
                let short_prof: Profile = serde_json::from_value(s.profile_data)?;
                let medium_prof: Profile = serde_json::from_value(m.profile_data)?;
                let long_prof: Profile = serde_json::from_value(l.profile_data)?;
                Ok(Some(TemporalLayers::new(short_prof, medium_prof, long_prof)))
            }
            _ => Ok(None),
        }
    }
}
