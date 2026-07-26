use crate::adapters::{IngestionBatch, LogAdapter};
use crate::domain::{Edge, Event};
use crate::errors::Result;
use crate::normalization::{parse_timestamp, IdentityResolver};
use chrono::Utc;
use tracing::debug;

pub struct SyslogAuthAdapter;

impl SyslogAuthAdapter {
    pub fn new() -> Self {
        Self
    }

    fn parse_line(&self, line: &str, resolver: &IdentityResolver) -> Option<(Vec<crate::domain::Entity>, Event, Vec<Edge>)> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Standard Syslog format: <Timestamp (15 chars e.g. "Jul 26 18:00:00")> <Hostname> <Process[PID]>: <Message>
        // Or RFC5424 / modern syslog formats
        let (timestamp, rest) = if trimmed.len() >= 15 && parse_timestamp(&trimmed[..15]).is_some() {
            (parse_timestamp(&trimmed[..15]).unwrap(), &trimmed[15..])
        } else {
            (Utc::now(), trimmed)
        };

        let rest_trimmed = rest.trim_start();
        
        // Extract Hostname & Process if present
        let mut parts = rest_trimmed.splitn(2, ':');
        let header = parts.next().unwrap_or("");
        let message = parts.next().unwrap_or(rest_trimmed).trim();

        let header_parts: Vec<&str> = header.split_whitespace().collect();
        let host_name = if !header_parts.is_empty() { header_parts[0] } else { "localhost" };
        let process = if header_parts.len() > 1 { header_parts[1] } else { "auth" };

        let host_entity = resolver.resolve_entity(host_name, "host", serde_json::json!({"process": process}), timestamp);

        // Classify authentication actions from line patterns
        if message.contains("Accepted password") || message.contains("Accepted publickey") {
            // SSH Success: Accepted password for <user> from <ip> port <port>
            let user = extract_between(message, "for ", " from").unwrap_or_else(|| "unknown".to_string());
            let ip = extract_after(message, "from ")
                .map(|s| s.split_whitespace().next().unwrap_or("0.0.0.0").to_string())
                .unwrap_or_else(|| "0.0.0.0".to_string());

            let user_entity = resolver.resolve_entity(&user, "user", serde_json::json!({}), timestamp);
            let ip_entity = resolver.resolve_entity(&ip, "ip_address", serde_json::json!({}), timestamp);

            let edge1 = Edge::new(user_entity.id, host_entity.id, "logged_into", timestamp);
            let edge2 = Edge::new(user_entity.id, ip_entity.id, "connected_from", timestamp);

            let context = serde_json::json!({
                "action": "ssh_login_success",
                "raw_message": message,
                "ip": ip,
                "user": user
            });

            let event = Event::new("ssh_login_success", timestamp, user_entity.id, Some(host_entity.id), context);

            Some((vec![host_entity, user_entity, ip_entity], event, vec![edge1, edge2]))
        } else if message.contains("Failed password") || message.contains("authentication failure") {
            // SSH Failure: Failed password for <invalid user> <user> from <ip>
            let user = if message.contains("for invalid user ") {
                extract_between(message, "for invalid user ", " from").unwrap_or_else(|| "invalid".to_string())
            } else {
                extract_between(message, "for ", " from").unwrap_or_else(|| "unknown".to_string())
            };
            let ip = extract_after(message, "from ")
                .map(|s| s.split_whitespace().next().unwrap_or("0.0.0.0").to_string())
                .unwrap_or_else(|| "0.0.0.0".to_string());

            let user_entity = resolver.resolve_entity(&user, "user", serde_json::json!({}), timestamp);
            let ip_entity = resolver.resolve_entity(&ip, "ip_address", serde_json::json!({}), timestamp);

            let edge = Edge::new(user_entity.id, ip_entity.id, "failed_login_from", timestamp);

            let context = serde_json::json!({
                "action": "auth_failed",
                "raw_message": message,
                "ip": ip,
                "user": user
            });

            let event = Event::new("auth_failed", timestamp, user_entity.id, Some(host_entity.id), context);

            Some((vec![host_entity, user_entity, ip_entity], event, vec![edge]))
        } else if message.contains("sudo:") || message.contains("COMMAND=") {
            // sudo execution: sudo: <user> : TTY=... ; USER=<target_user> ; COMMAND=<cmd>
            let user = message.split(':').next().unwrap_or("unknown").trim();
            let target_user = extract_between(message, "USER=", " ;").unwrap_or_else(|| "root".to_string());
            let command = extract_after(message, "COMMAND=").unwrap_or_default();

            let user_entity = resolver.resolve_entity(user, "user", serde_json::json!({}), timestamp);
            let target_user_entity = resolver.resolve_entity(&target_user, "user", serde_json::json!({}), timestamp);

            let edge = Edge::new(user_entity.id, target_user_entity.id, "escalated_to", timestamp);

            let context = serde_json::json!({
                "action": "sudo_exec",
                "raw_message": message,
                "command": command,
                "target_user": target_user
            });

            let event = Event::new("sudo_exec", timestamp, user_entity.id, Some(target_user_entity.id), context);

            Some((vec![host_entity, user_entity, target_user_entity], event, vec![edge]))
        } else {
            // Generic Auth Syslog entry
            let context = serde_json::json!({
                "action": "syslog_auth_event",
                "raw_message": message,
                "process": process
            });

            let event = Event::new("syslog_auth_event", timestamp, host_entity.id, None, context);
            Some((vec![host_entity], event, vec![]))
        }
    }
}

fn extract_between(text: &str, start: &str, end: &str) -> Option<String> {
    let s_idx = text.find(start)? + start.len();
    let rest = &text[s_idx..];
    let e_idx = rest.find(end)?;
    Some(rest[..e_idx].trim().to_string())
}

fn extract_after(text: &str, start: &str) -> Option<String> {
    let s_idx = text.find(start)? + start.len();
    Some(text[s_idx..].trim().to_string())
}

impl LogAdapter for SyslogAuthAdapter {
    fn name(&self) -> &str {
        "syslog_auth"
    }

    fn can_parse(&self, sample: &str) -> bool {
        sample.contains("sshd")
            || sample.contains("sudo")
            || sample.contains("pam_unix")
            || sample.contains("Accepted password")
            || sample.contains("Failed password")
    }

    fn parse(&self, content: &str, resolver: &IdentityResolver) -> Result<IngestionBatch> {
        let mut batch = IngestionBatch::new();

        for line in content.lines() {
            if let Some((entities, event, edges)) = self.parse_line(line, resolver) {
                batch.entities.extend(entities);
                batch.events.push(event);
                batch.edges.extend(edges);
            }
        }

        debug!("SyslogAuthAdapter parsed {} events", batch.events.len());
        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_accepted() {
        let adapter = SyslogAuthAdapter::new();
        let resolver = IdentityResolver::default();
        let log_line = "Jul 26 18:00:00 myhost sshd[1234]: Accepted password for alice from 192.168.1.50 port 54321 ssh2";

        let batch = adapter.parse(log_line, &resolver).unwrap();
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].event_type, "ssh_login_success");
        assert_eq!(batch.entities.len(), 3); // host, user, ip
        assert_eq!(batch.edges.len(), 2);
    }

    #[test]
    fn test_parse_sudo_exec() {
        let adapter = SyslogAuthAdapter::new();
        let resolver = IdentityResolver::default();
        let log_line = "Jul 26 18:02:00 myhost sudo: alice : TTY=pts/0 ; PWD=/home/alice ; USER=root ; COMMAND=/bin/cat /etc/shadow";

        let batch = adapter.parse(log_line, &resolver).unwrap();
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].event_type, "sudo_exec");
    }
}
