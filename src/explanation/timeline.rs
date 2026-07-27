use crate::domain::{Entity, Event};
use crate::repository::{EdgeRepository, EventRepository};
use crate::storage::sqlite::SqliteStorage;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct TimelineGenerator;

impl TimelineGenerator {
    /// Render a chronological narrative timeline of an entity's behavioral history
    pub fn build_timeline(entity: &Entity, storage: &SqliteStorage) -> crate::errors::Result<String> {
        let events = EventRepository::find_by_entity(storage, &entity.id, DateTime::<Utc>::MIN_UTC)?;
        let edges = EdgeRepository::find_edges_for_entity(storage, &entity.id)?;

        let mut out = String::new();
        out.push_str(&format!("========================================\n"));
        out.push_str(&format!(" Behavioral Timeline: {} ({})\n", entity.canonical_name, entity.entity_type));
        out.push_str(&format!("========================================\n"));

        if events.is_empty() && edges.is_empty() {
            out.push_str("No recorded behavioral history for this entity.\n");
            return Ok(out);
        }

        // Group events by date (YYYY-MM-DD)
        let mut daily_events: HashMap<String, Vec<&Event>> = HashMap::new();
        for ev in &events {
            let day_key = ev.timestamp.format("%Y-%m-%d").to_string();
            daily_events.entry(day_key).or_default().push(ev);
        }

        let mut sorted_days: Vec<String> = daily_events.keys().cloned().collect();
        sorted_days.sort();

        out.push_str(&format!("Total Observed Events: {}\n", events.len()));
        out.push_str(&format!("Active Timeline Windows: {}\n\n", sorted_days.len()));

        out.push_str("Chronological Phases & Key Activities:\n");
        for (idx, day) in sorted_days.iter().enumerate() {
            let day_evs = &daily_events[day];
            let mut action_counts: HashMap<&str, usize> = HashMap::new();
            for ev in day_evs {
                *action_counts.entry(&ev.event_type).or_insert(0) += 1;
            }

            let actions_summary: Vec<String> = action_counts
                .iter()
                .map(|(act, cnt)| format!("{} (x{})", act, cnt))
                .collect();

            out.push_str(&format!(
                " Phase {}: [{}] — {} events\n   Actions: {}\n",
                idx + 1,
                day,
                day_evs.len(),
                actions_summary.join(", ")
            ));
        }

        if !edges.is_empty() {
            out.push_str("\nKnown Graph Relationships:\n");
            for edge in &edges {
                let role = if edge.source_id == entity.id { "Source -> Target" } else { "Target <- Source" };
                out.push_str(&format!(
                    " • Relationship '{}' ({}) | First seen: {} | Last seen: {}\n",
                    edge.rel_type,
                    role,
                    edge.first_seen.format("%Y-%m-%d").to_string(),
                    edge.last_seen.format("%Y-%m-%d").to_string()
                ));
            }
        }

        out.push_str("========================================\n");
        Ok(out)
    }
}
