use crate::errors::{LafufuError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub identity_resolver: IdentityResolverConfig,
    #[serde(default)]
    pub detection: DetectionConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DatabaseConfig {
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct IdentityResolverConfig {
    pub mappings: HashMap<String, String>, // alias -> canonical_name
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectionConfig {
    pub sequence_weight: f64,
    pub velocity_weight: f64,
    pub graph_weight: f64,
    pub threshold: f64,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            sequence_weight: 0.4,
            velocity_weight: 0.3,
            graph_weight: 0.3,
            threshold: 0.7,
        }
    }
}

impl AppConfig {
    /// Load the configuration from the default path ~/.config/lafufu/lafufu.yaml
    pub fn load_default() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lafufu");
        let config_path = config_dir.join("lafufu.yaml");

        if !config_path.exists() {
            info!("Configuration file not found. Creating default config at: {:?}", config_path);
            fs::create_dir_all(&config_dir)?;
            let default_config = Self::default();
            let default_yaml = serde_yaml::to_string(&default_config)
                .map_err(|e| LafufuError::Config(e.to_string()))?;
            fs::write(&config_path, default_yaml)?;
            return Ok(default_config);
        }

        debug!("Loading configuration from: {:?}", config_path);
        let yaml_content = fs::read_to_string(&config_path)?;
        let config: AppConfig = serde_yaml::from_str(&yaml_content)
            .map_err(|e| LafufuError::Config(format!("Failed to parse configuration: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Resolve the database directory path, ensuring the parent directories exist
    pub fn get_db_path(&self) -> Result<PathBuf> {
        let db_path = if let Some(ref path) = self.database.path {
            PathBuf::from(path)
        } else {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("lafufu")
                .join("lafufu.db")
        };

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(db_path)
    }

    /// Validate configuration invariants on startup
    pub fn validate(&self) -> Result<()> {
        // Validate weights
        let total_weight = self.detection.sequence_weight 
            + self.detection.velocity_weight 
            + self.detection.graph_weight;
            
        if (total_weight - 1.0).abs() > 1e-4 {
            return Err(LafufuError::Config(format!(
                "Detection weights must sum up to 1.0 (currently sums to {:.2})", 
                total_weight
            )));
        }

        if self.detection.threshold < 0.0 || self.detection.threshold > 1.0 {
            return Err(LafufuError::Config(
                "Detection threshold must be between 0.0 and 1.0".to_string()
            ));
        }

        // Validate identity mappings to prevent cycles or empty keys/values
        for (alias, canonical) in &self.identity_resolver.mappings {
            if alias.trim().is_empty() || canonical.trim().is_empty() {
                return Err(LafufuError::Config(
                    "Identity mappings cannot contain empty aliases or canonical names".to_string()
                ));
            }
            if alias == canonical {
                return Err(LafufuError::Config(format!(
                    "Identity mapping for '{}' is a self-reference", alias
                )));
            }
            // Simple cycle detection (alias -> canonical -> alias)
            if let Some(next_canonical) = self.identity_resolver.mappings.get(canonical) {
                if next_canonical == alias {
                    return Err(LafufuError::Config(format!(
                        "Cyclic identity mapping detected: {} <-> {}", alias, canonical
                    )));
                }
            }
        }

        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            identity_resolver: IdentityResolverConfig::default(),
            detection: DetectionConfig::default(),
        }
    }
}

// Module for fetching user directories
mod dirs {
    use std::path::PathBuf;
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")).ok()
    }
    pub fn data_dir() -> Option<PathBuf> {
        std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")).ok()
    }
}
