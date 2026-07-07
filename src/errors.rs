use thiserror::Error;

#[derive(Error, Debug)]
pub enum LafufuError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Log ingestion/parsing error: {0}")]
    Ingestion(String),

    #[error("Identity resolution error: {0}")]
    IdentityResolution(String),

    #[error("Behavioral profiling error: {0}")]
    Analysis(String),

    #[error("Serialization/Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, LafufuError>;
