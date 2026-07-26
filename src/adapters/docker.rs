use crate::adapters::{IngestionBatch, LogAdapter};
use crate::domain::{Edge, Event};
use crate::errors::Result;
use crate::normalization::{parse_timestamp, IdentityResolver};
use chrono::Utc;
use serde_json::Value;

pub struct DockerEventsAdapter;

impl DockerEventsAdapter {
    pub fn new() -> Self {
        Self
    }

    fn parse_docker_event(&self, val: &Value, resolver: &IdentityResolver) -> Option<(Vec<crate::domain::Entity>, Event, Vec<Edge>)> {
        let obj = val.as_object()?;

        // 1. Timestamp
        let timestamp = obj.get("time")
            .or_else(|| obj.get("timeNano"))
            .and_then(|v| v.as_i64().and_then(|i| parse_timestamp(&i.to_string())))
            .unwrap_or_else(Utc::now);

        // 2. Action / Event Type
        let raw_action = obj.get("action")
            .or_else(|| obj.get("Action"))
            .and_then(|v| v.as_str())
            .unwrap_or("container_event");

        let event_type = format!("docker_{}", raw_action.replace(':', "_"));

        // 3. Container ID / Name & Image
        let attributes = obj.get("Actor").and_then(|a| a.get("Attributes")).cloned().unwrap_or(serde_json::json!({}));
        
        let container_name = attributes.get("name")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("id").and_then(|v| v.as_str()))
            .unwrap_or("unknown_container")
            .to_string();

        let image_name = attributes.get("image")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("from").and_then(|v| v.as_str()))
            .unwrap_or("unknown_image")
            .to_string();

        let container_entity = resolver.resolve_entity(&container_name, "container", attributes, timestamp);
        let image_entity = resolver.resolve_entity(&image_name, "container_image", serde_json::json!({}), timestamp);

        let edge = Edge::new(container_entity.id, image_entity.id, "instantiated_from", timestamp);

        let context = serde_json::json!({
            "action": raw_action,
            "container": container_name,
            "image": image_name,
            "raw_event": val
        });

        let event = Event::new(&event_type, timestamp, container_entity.id, Some(image_entity.id), context);

        Some((vec![container_entity, image_entity], event, vec![edge]))
    }
}

impl LogAdapter for DockerEventsAdapter {
    fn name(&self) -> &str {
        "docker_events"
    }

    fn can_parse(&self, sample: &str) -> bool {
        (sample.contains("Type") || sample.contains("type"))
            && (sample.contains("container") || sample.contains("Container") || sample.contains("dockerd"))
    }

    fn parse(&self, content: &str, resolver: &IdentityResolver) -> Result<IngestionBatch> {
        let mut batch = IngestionBatch::new();
        let trimmed = content.trim();

        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            match val {
                Value::Array(arr) => {
                    for item in arr {
                        if let Some((entities, event, edges)) = self.parse_docker_event(&item, resolver) {
                            batch.entities.extend(entities);
                            batch.events.push(event);
                            batch.edges.extend(edges);
                        }
                    }
                    return Ok(batch);
                }
                Value::Object(_) => {
                    if let Some((entities, event, edges)) = self.parse_docker_event(&val, resolver) {
                        batch.entities.extend(entities);
                        batch.events.push(event);
                        batch.edges.extend(edges);
                    }
                    return Ok(batch);
                }
                _ => {}
            }
        }

        for line in content.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(line_trimmed) {
                if let Some((entities, event, edges)) = self.parse_docker_event(&val, resolver) {
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
    fn test_parse_docker_event() {
        let adapter = DockerEventsAdapter::new();
        let resolver = IdentityResolver::default();
        let payload = r#"{
            "status": "start",
            "id": "c123456789",
            "from": "nginx:latest",
            "Type": "container",
            "action": "start",
            "Actor": {
                "ID": "c123456789",
                "Attributes": {
                    "image": "nginx:latest",
                    "name": "web_server"
                }
            },
            "time": 1719829200
        }"#;

        let batch = adapter.parse(payload, &resolver).unwrap();
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].event_type, "docker_start");
        assert_eq!(batch.entities.len(), 2); // web_server container, nginx:latest image
    }
}
