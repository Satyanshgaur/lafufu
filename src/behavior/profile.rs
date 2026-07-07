use crate::behavior::statistics::CategoricalHistogram;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Profile {
    User(UserProfile),
    Service(ServiceProfile),
    Container(ContainerProfile),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserProfile {
    pub login_hours: CategoricalHistogram,
    pub locations: CategoricalHistogram,
    pub session_durations: Vec<f64>, // List of recent session lengths for variance computation
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServiceProfile {
    pub egress_endpoints: CategoricalHistogram,
    pub api_endpoints: CategoricalHistogram,
    pub error_rates: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ContainerProfile {
    pub process_names: CategoricalHistogram,
    pub network_ports: CategoricalHistogram,
}
