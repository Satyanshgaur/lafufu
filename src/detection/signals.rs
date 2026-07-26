use crate::behavior::profile::Profile;
use crate::behavior::statistics::RunningStats;
use crate::domain::Event;

/// Sequence Signal (S_seq): Evaluates how unexpected an event is given historical action, time, and location distributions
pub struct SequenceSignal;

impl SequenceSignal {
    pub fn score(event: &Event, profile: &Profile) -> f64 {
        let hour_bin = event.timestamp.format("%H").to_string();

        let (action_prob, hour_prob, loc_prob) = match profile {
            Profile::User(p) => (
                p.actions.smoothed_probability(&event.event_type),
                p.login_hours.smoothed_probability(&hour_bin),
                event.context.get("ip")
                    .and_then(|v| v.as_str())
                    .map(|ip| p.locations.smoothed_probability(ip))
                    .unwrap_or(0.5),
            ),
            Profile::Service(p) => (
                p.action_frequencies.smoothed_probability(&event.event_type),
                p.hourly_activity.smoothed_probability(&hour_bin),
                1.0,
            ),
            Profile::Container(p) => (
                p.actions.smoothed_probability(&event.event_type),
                p.hourly_activity.smoothed_probability(&hour_bin),
                1.0,
            ),
            Profile::Repository(p) => (
                p.actions.smoothed_probability(&event.event_type),
                p.hourly_activity.smoothed_probability(&hour_bin),
                1.0,
            ),
            Profile::Generic(p) => (
                p.actions.smoothed_probability(&event.event_type),
                p.hourly_activity.smoothed_probability(&hour_bin),
                1.0,
            ),
        };

        // Combined probability product
        let combined_prob = action_prob * hour_prob * loc_prob;
        // Convert to anomaly score: 1.0 - combined_prob, normalized between 0.0 and 1.0
        (1.0 - combined_prob).clamp(0.0, 1.0)
    }
}

/// Velocity Signal (S_vel): Evaluates sudden rate/burst spikes compared to normal frequency baselines
pub struct VelocitySignal;

impl VelocitySignal {
    pub fn score(event_rate: f64, baseline_stats: &RunningStats) -> f64 {
        if baseline_stats.count < 2 {
            return 0.0;
        }

        let z = baseline_stats.z_score(event_rate);
        if z <= 0.0 {
            return 0.0;
        }

        // Sigmoid transform of positive z-score into [0.0, 1.0] range
        // Z-score of 3.0 maps to ~0.95 anomaly score
        let sigmoid = 1.0 / (1.0 + (-0.8 * (z - 2.0)).exp());
        sigmoid.clamp(0.0, 1.0)
    }
}

/// Graph Signal (S_graph): Evaluates changes in graph topology (interacting with unobserved entities or new edges)
pub struct GraphSignal;

impl GraphSignal {
    pub fn score(target_id_str: Option<&str>, _edge_rel: Option<&str>, profile: &Profile) -> f64 {
        let Some(target) = target_id_str else {
            return 0.0;
        };

        let target_prob = match profile {
            Profile::User(p) => p.interaction_partners.raw_probability(target),
            Profile::Service(p) => p.egress_endpoints.raw_probability(target),
            Profile::Container(p) => p.network_ports.raw_probability(target),
            Profile::Repository(p) => p.contributors.raw_probability(target),
            Profile::Generic(p) => p.targets.raw_probability(target),
        };

        if target_prob == 0.0 {
            // Unobserved interaction partner -> high graph anomaly (1.0)
            1.0
        } else {
            // Observed partner -> anomaly score inversely proportional to frequency
            (1.0 - target_prob).clamp(0.0, 0.8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::profile::UserProfile;

    #[test]
    fn test_graph_signal_unseen_target() {
        let mut u = UserProfile::default();
        u.interaction_partners.observe("target_a");
        let profile = Profile::User(u);

        // Target A is known
        let score_known = GraphSignal::score(Some("target_a"), None, &profile);
        assert!(score_known < 0.5);

        // Target B is completely new
        let score_new = GraphSignal::score(Some("target_b"), None, &profile);
        assert_eq!(score_new, 1.0);
    }

    #[test]
    fn test_velocity_signal_spike() {
        let mut stats = RunningStats::new();
        for _ in 0..10 {
            stats.update(5.0);
        }

        // Rate = 5.0 is normal -> 0.0 score
        assert_eq!(VelocitySignal::score(5.0, &stats), 0.0);

        // Rate = 50.0 is huge spike -> high anomaly score near 1.0
        let score_spike = VelocitySignal::score(50.0, &stats);
        assert!(score_spike > 0.8);
    }
}
