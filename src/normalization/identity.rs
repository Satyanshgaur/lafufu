use crate::domain::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Configuration for identity mappings and entity resolution rules
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentityConfig {
    /// Explicit alias mapping: raw identifier string -> canonical name
    pub alias_map: HashMap<String, String>,
    /// Type overrides: canonical name -> entity type
    pub type_map: HashMap<String, String>,
}

pub struct IdentityResolver {
    config: RwLock<IdentityConfig>,
}

impl Default for IdentityResolver {
    fn default() -> Self {
        Self::new(IdentityConfig::default())
    }
}

impl IdentityResolver {
    pub fn new(config: IdentityConfig) -> Self {
        Self {
            config: RwLock::new(config),
        }
    }

    /// Add an alias mapping at runtime (e.g. "alice@company.com" -> "alice")
    pub fn register_alias(&self, raw_alias: &str, canonical_name: &str) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.alias_map.insert(raw_alias.to_string(), canonical_name.to_string());
        }
    }

    /// Resolve a raw identifier and default entity type to a canonical name and normalized type
    pub fn resolve_identifier(&self, raw_id: &str, default_type: &str) -> (String, String) {
        let raw_clean = raw_id.trim();

        // 1. Check explicit alias map
        let canonical_name = if let Ok(cfg) = self.config.read() {
            if let Some(canonical) = cfg.alias_map.get(raw_clean) {
                canonical.clone()
            } else {
                Self::heuristic_canonical_name(raw_clean)
            }
        } else {
            Self::heuristic_canonical_name(raw_clean)
        };

        // 2. Check explicit type map or deduce type
        let entity_type = if let Ok(cfg) = self.config.read() {
            if let Some(t) = cfg.type_map.get(&canonical_name) {
                t.clone()
            } else {
                Self::heuristic_entity_type(&canonical_name, default_type)
            }
        } else {
            Self::heuristic_entity_type(&canonical_name, default_type)
        };

        (canonical_name, entity_type)
    }

    /// Resolve raw inputs directly into a domain Entity instance
    pub fn resolve_entity(
        &self,
        raw_id: &str,
        default_type: &str,
        attributes: serde_json::Value,
        valid_from: DateTime<Utc>,
    ) -> Entity {
        let (canonical_name, entity_type) = self.resolve_identifier(raw_id, default_type);
        Entity::new(&entity_type, &canonical_name, attributes, valid_from)
    }

    /// Automatic heuristic normalization for canonical names
    fn heuristic_canonical_name(raw: &str) -> String {
        // If email (e.g. "alice@company.com"), extract username part as candidate
        if raw.contains('@') && !raw.starts_with('@') {
            if let Some((user, _domain)) = raw.split_once('@') {
                return user.to_lowercase();
            }
        }

        // Clean up common prefixes like "uid=1001" -> "1001" or user domains
        if let Some(suffix) = raw.strip_prefix("uid=") {
            return suffix.to_string();
        }

        raw.to_lowercase()
    }

    /// Heuristic determination of entity types
    fn heuristic_entity_type(canonical_name: &str, fallback_type: &str) -> String {
        if fallback_type != "unknown" && !fallback_type.is_empty() {
            return fallback_type.to_string();
        }

        // Detect IP addresses
        if canonical_name.parse::<std::net::IpAddr>().is_ok() {
            return "ip_address".to_string();
        }

        // Default to provided fallback
        fallback_type.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_canonicalization() {
        let resolver = IdentityResolver::default();
        let (canonical, entity_type) = resolver.resolve_identifier("Alice.Smith@company.com", "user");
        assert_eq!(canonical, "alice.smith");
        assert_eq!(entity_type, "user");
    }

    #[test]
    fn test_explicit_alias() {
        let resolver = IdentityResolver::default();
        resolver.register_alias("uid=1001", "alice");
        let (canonical, entity_type) = resolver.resolve_identifier("uid=1001", "user");
        assert_eq!(canonical, "alice");
        assert_eq!(entity_type, "user");
    }

    #[test]
    fn test_ip_detection() {
        let resolver = IdentityResolver::default();
        let (canonical, entity_type) = resolver.resolve_identifier("192.168.1.100", "unknown");
        assert_eq!(canonical, "192.168.1.100");
        assert_eq!(entity_type, "ip_address");
    }
}
