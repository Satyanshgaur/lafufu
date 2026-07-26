use crate::adapters::{IngestionBatch, LogAdapter};
use crate::domain::{Edge, Event};
use crate::errors::Result;
use crate::normalization::{parse_timestamp, IdentityResolver};
use chrono::Utc;
use serde_json::Value;

pub struct GitHubEventsAdapter;

impl GitHubEventsAdapter {
    pub fn new() -> Self {
        Self
    }

    fn parse_event_json(&self, val: &Value, resolver: &IdentityResolver) -> Option<(Vec<crate::domain::Entity>, Event, Vec<Edge>)> {
        let obj = val.as_object()?;

        // 1. Extract GitHub timestamp
        let timestamp = obj.get("created_at")
            .or_else(|| obj.get("updated_at"))
            .or_else(|| obj.get("pushed_at"))
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp)
            .unwrap_or_else(Utc::now);

        // 2. Extract Actor / Sender User
        let actor_login = obj.get("sender")
            .and_then(|s| s.get("login"))
            .or_else(|| obj.get("actor").and_then(|a| a.get("login").or_else(|| a.get("display_login"))))
            .or_else(|| obj.get("pusher").and_then(|p| p.get("name")))
            .and_then(|v| v.as_str())
            .unwrap_or("github_bot");

        let user_entity = resolver.resolve_entity(actor_login, "user", serde_json::json!({"platform": "github"}), timestamp);

        // 3. Extract Repository Entity
        let repo_name = obj.get("repository")
            .and_then(|r| r.get("full_name").or_else(|| r.get("name")))
            .or_else(|| obj.get("repo").and_then(|r| r.get("name")))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_repo");

        let repo_entity = resolver.resolve_entity(repo_name, "github_repo", serde_json::json!({}), timestamp);

        // 4. Extract Event Type
        let raw_type = obj.get("type")
            .or_else(|| obj.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("github_activity");

        let event_type = if obj.contains_key("commits") || raw_type.contains("Push") {
            "github_push"
        } else if obj.contains_key("pull_request") || raw_type.contains("PullRequest") {
            "github_pull_request"
        } else if obj.contains_key("issue") || raw_type.contains("Issue") {
            "github_issue"
        } else if obj.contains_key("workflow_run") || raw_type.contains("Workflow") {
            "github_workflow_run"
        } else {
            raw_type
        };

        // 5. Build Edge between User and Repository
        let edge_type = match event_type {
            "github_push" => "pushed_to",
            "github_pull_request" => "opened_pr_in",
            "github_issue" => "interacted_with_issue",
            "github_workflow_run" => "triggered_workflow",
            _ => "acted_on",
        };

        let edge = Edge::new(user_entity.id, repo_entity.id, edge_type, timestamp);

        let context = serde_json::json!({
            "event_type": event_type,
            "raw_payload": val,
            "repo": repo_name,
            "actor": actor_login
        });

        let event = Event::new(event_type, timestamp, user_entity.id, Some(repo_entity.id), context);

        Some((vec![user_entity, repo_entity], event, vec![edge]))
    }
}

impl LogAdapter for GitHubEventsAdapter {
    fn name(&self) -> &str {
        "github_events"
    }

    fn can_parse(&self, sample: &str) -> bool {
        (sample.contains("repository") || sample.contains("repo"))
            && (sample.contains("sender") || sample.contains("pusher") || sample.contains("actor"))
    }

    fn parse(&self, content: &str, resolver: &IdentityResolver) -> Result<IngestionBatch> {
        let mut batch = IngestionBatch::new();
        let trimmed = content.trim();

        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            match val {
                Value::Array(arr) => {
                    for item in arr {
                        if let Some((entities, event, edges)) = self.parse_event_json(&item, resolver) {
                            batch.entities.extend(entities);
                            batch.events.push(event);
                            batch.edges.extend(edges);
                        }
                    }
                    return Ok(batch);
                }
                Value::Object(_) => {
                    if let Some((entities, event, edges)) = self.parse_event_json(&val, resolver) {
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
                if let Some((entities, event, edges)) = self.parse_event_json(&val, resolver) {
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
    fn test_parse_github_push_event() {
        let adapter = GitHubEventsAdapter::new();
        let resolver = IdentityResolver::default();
        let payload = r#"{
            "type": "PushEvent",
            "created_at": "2026-07-26T18:00:00Z",
            "actor": {"login": "octocat"},
            "repo": {"name": "octocat/Hello-World"},
            "payload": {"commits": [{"sha": "12345"}]}
        }"#;

        let batch = adapter.parse(payload, &resolver).unwrap();
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].event_type, "github_push");
        assert_eq!(batch.entities.len(), 2); // octocat user, octocat/Hello-World repo
        assert_eq!(batch.edges[0].rel_type, "pushed_to");
    }
}
