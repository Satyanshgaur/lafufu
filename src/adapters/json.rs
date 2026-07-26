use crate::adapters::{IngestionBatch, LogAdapter};
use crate::domain::{Edge, Event};
use crate::errors::Result;
use crate::normalization::{parse_timestamp, IdentityResolver};
use chrono::Utc;
use serde_json::Value;

pub struct JsonLogAdapter;

impl JsonLogAdapter {
    pub fn new() -> Self {
        Self
    }

    fn parse_json_object(&self, obj: &Value, resolver: &IdentityResolver) -> Option<(Vec<crate::domain::Entity>, Event, Vec<Edge>)> {
        let map = obj.as_object()?;

        // 1. Extract Timestamp
        let timestamp = map.iter()
            .find_map(|(k, v)| {
                let k_lower = k.to_lowercase();
                if k_lower == "timestamp" || k_lower == "time" || k_lower == "ts" || k_lower == "@timestamp" || k_lower == "date" {
                    v.as_str().and_then(parse_timestamp)
                        .or_else(|| v.as_i64().and_then(|i| parse_timestamp(&i.to_string())))
                        .or_else(|| v.as_f64().and_then(|f| parse_timestamp(&f.to_string())))
                } else {
                    None
                }
            })
            .unwrap_or_else(Utc::now);

        // 2. Extract Event Type
        let event_type = map.iter()
            .find_map(|(k, v)| {
                let k_lower = k.to_lowercase();
                if k_lower == "event" || k_lower == "event_type" || k_lower == "action" || k_lower == "type" || k_lower == "message" {
                    v.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "generic_json_event".to_string());

        // 3. Extract Source Entity
        let (source_name, source_type) = map.iter()
            .find_map(|(k, v)| {
                let k_lower = k.to_lowercase();
                if k_lower == "user" || k_lower == "username" || k_lower == "actor" || k_lower == "source" || k_lower == "service" || k_lower == "app" {
                    v.as_str().map(|s| (s, if k_lower == "service" || k_lower == "app" { "service" } else { "user" }))
                } else {
                    None
                }
            })
            .unwrap_or(("system", "service"));

        let source_entity = resolver.resolve_entity(source_name, source_type, serde_json::json!({}), timestamp);

        // 4. Extract Target Entity (Optional)
        let target_info = map.iter()
            .find_map(|(k, v)| {
                let k_lower = k.to_lowercase();
                if k_lower == "target" || k_lower == "dest" || k_lower == "destination" || k_lower == "resource" || k_lower == "server" {
                    v.as_str().map(|s| (s, "resource"))
                } else {
                    None
                }
            });

        let mut entities = vec![source_entity.clone()];
        let mut edges = Vec::new();
        let target_id = if let Some((target_name, target_type)) = target_info {
            let target_entity = resolver.resolve_entity(target_name, target_type, serde_json::json!({}), timestamp);
            let edge = Edge::new(source_entity.id, target_entity.id, "interacted_with", timestamp);
            edges.push(edge);
            entities.push(target_entity.clone());
            Some(target_entity.id)
        } else {
            None
        };

        // 5. Build Event with full context payload
        let event = Event::new(&event_type, timestamp, source_entity.id, target_id, obj.clone());

        Some((entities, event, edges))
    }
}

impl LogAdapter for JsonLogAdapter {
    fn name(&self) -> &str {
        "generic_json"
    }

    fn can_parse(&self, sample: &str) -> bool {
        let trimmed = sample.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || serde_json::from_str::<Value>(trimmed).is_ok()
    }

    fn parse(&self, content: &str, resolver: &IdentityResolver) -> Result<IngestionBatch> {
        let mut batch = IngestionBatch::new();

        // Check if content is a JSON array
        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(content) {
            for item in arr {
                if let Some((entities, event, edges)) = self.parse_json_object(&item, resolver) {
                    batch.entities.extend(entities);
                    batch.events.push(event);
                    batch.edges.extend(edges);
                }
            }
            return Ok(batch);
        }

        // Process line by line for NDJSON / JSON lines
        for line in content.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(line_trimmed) {
                if let Some((entities, event, edges)) = self.parse_json_object(&val, resolver) {
                    batch.entities.extend(entities);
                    batch.events.push(event);
                    batch.edges.extend(edges);
                }
            }
        }

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_log_lines() {
        let adapter = JsonLogAdapter::new();
        let resolver = IdentityResolver::default();
        let json_data = r#"{"timestamp": "2026-07-26T18:00:00Z", "user": "alice", "action": "login", "target": "auth_server"}
{"timestamp": "2026-07-26T18:05:00Z", "user": "bob", "action": "file_access", "target": "secret.txt"}"#;

        let batch = adapter.parse(json_data, &resolver).unwrap();
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.entities.len(), 4); // alice, auth_server, bob, secret.txt
        assert_eq!(batch.edges.len(), 2);
    }
}
