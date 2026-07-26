use crate::behavior::statistics::{CategoricalHistogram, RunningStats};
use crate::domain::{Edge, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Profile {
    User(UserProfile),
    Service(ServiceProfile),
    Container(ContainerProfile),
    Repository(RepositoryProfile),
    Generic(GenericProfile),
}

impl Default for Profile {
    fn default() -> Self {
        Profile::Generic(GenericProfile::default())
    }
}

impl Profile {
    /// Create an empty profile tailored to entity type
    pub fn for_entity_type(entity_type: &str) -> Self {
        match entity_type.to_lowercase().as_str() {
            "user" => Profile::User(UserProfile::default()),
            "service" => Profile::Service(ServiceProfile::default()),
            "container" => Profile::Container(ContainerProfile::default()),
            "github_repo" | "repository" | "repo" => Profile::Repository(RepositoryProfile::default()),
            _ => Profile::Generic(GenericProfile::default()),
        }
    }

    /// Update profile with a new Event observation
    pub fn observe_event(&mut self, event: &Event, is_source: bool) {
        let hour_bin = event.timestamp.format("%H").to_string(); // "00" to "23"

        match self {
            Profile::User(p) => {
                p.login_hours.observe(&hour_bin);
                p.actions.observe(&event.event_type);
                if let Some(ip) = event.context.get("ip").and_then(|v| v.as_str()) {
                    p.locations.observe(ip);
                }
            }
            Profile::Service(p) => {
                p.action_frequencies.observe(&event.event_type);
                p.hourly_activity.observe(&hour_bin);
                if let Some(endpoint) = event.context.get("endpoint").or_else(|| event.context.get("target")).and_then(|v| v.as_str()) {
                    p.api_endpoints.observe(endpoint);
                }
            }
            Profile::Container(p) => {
                p.actions.observe(&event.event_type);
                p.hourly_activity.observe(&hour_bin);
                if let Some(cmd) = event.context.get("command").and_then(|v| v.as_str()) {
                    p.process_names.observe(cmd);
                }
            }
            Profile::Repository(p) => {
                p.actions.observe(&event.event_type);
                p.hourly_activity.observe(&hour_bin);
                if is_source {
                    if let Some(actor) = event.context.get("actor").and_then(|v| v.as_str()) {
                        p.contributors.observe(actor);
                    }
                }
            }
            Profile::Generic(p) => {
                p.actions.observe(&event.event_type);
                p.hourly_activity.observe(&hour_bin);
            }
        }
    }

    /// Update profile with an Edge relationship observation
    pub fn observe_edge(&mut self, edge: &Edge, is_source: bool) {
        let partner_id = if is_source { edge.target_id.to_string() } else { edge.source_id.to_string() };

        match self {
            Profile::User(p) => p.interaction_partners.observe(&partner_id),
            Profile::Service(p) => p.egress_endpoints.observe(&partner_id),
            Profile::Container(p) => p.network_ports.observe(&partner_id),
            Profile::Repository(p) => p.contributors.observe(&partner_id),
            Profile::Generic(p) => p.targets.observe(&partner_id),
        }
    }

    /// Compute Jensen-Shannon Divergence between two profiles of the same type
    pub fn jensen_shannon_divergence(&self, other: &Profile) -> f64 {
        match (self, other) {
            (Profile::User(p1), Profile::User(p2)) => {
                0.4 * p1.login_hours.jensen_shannon_divergence(&p2.login_hours)
                    + 0.3 * p1.actions.jensen_shannon_divergence(&p2.actions)
                    + 0.3 * p1.locations.jensen_shannon_divergence(&p2.locations)
            }
            (Profile::Service(p1), Profile::Service(p2)) => {
                0.5 * p1.action_frequencies.jensen_shannon_divergence(&p2.action_frequencies)
                    + 0.5 * p1.api_endpoints.jensen_shannon_divergence(&p2.api_endpoints)
            }
            (Profile::Container(p1), Profile::Container(p2)) => {
                0.5 * p1.process_names.jensen_shannon_divergence(&p2.process_names)
                    + 0.5 * p1.actions.jensen_shannon_divergence(&p2.actions)
            }
            (Profile::Repository(p1), Profile::Repository(p2)) => {
                0.5 * p1.actions.jensen_shannon_divergence(&p2.actions)
                    + 0.5 * p1.contributors.jensen_shannon_divergence(&p2.contributors)
            }
            (Profile::Generic(p1), Profile::Generic(p2)) => {
                0.5 * p1.actions.jensen_shannon_divergence(&p2.actions)
                    + 0.5 * p1.targets.jensen_shannon_divergence(&p2.targets)
            }
            _ => 1.0, // Different profile types divergence is maximum
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserProfile {
    pub login_hours: CategoricalHistogram,
    pub locations: CategoricalHistogram,
    pub actions: CategoricalHistogram,
    pub interaction_partners: CategoricalHistogram,
    pub session_stats: RunningStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServiceProfile {
    pub egress_endpoints: CategoricalHistogram,
    pub api_endpoints: CategoricalHistogram,
    pub action_frequencies: CategoricalHistogram,
    pub hourly_activity: CategoricalHistogram,
    pub error_stats: RunningStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ContainerProfile {
    pub process_names: CategoricalHistogram,
    pub network_ports: CategoricalHistogram,
    pub actions: CategoricalHistogram,
    pub hourly_activity: CategoricalHistogram,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RepositoryProfile {
    pub actions: CategoricalHistogram,
    pub contributors: CategoricalHistogram,
    pub hourly_activity: CategoricalHistogram,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GenericProfile {
    pub actions: CategoricalHistogram,
    pub targets: CategoricalHistogram,
    pub hourly_activity: CategoricalHistogram,
}
