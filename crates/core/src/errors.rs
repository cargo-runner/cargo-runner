#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("toml deserialize error: {0}")]
    Deserialize(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("network client error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("set default error for context: {0}")]
    SetDefault(crate::Context),
    #[error("Config Merge Conflict, Name doesnt match: {0} and {1}")]
    MergeConflict(String,String),

    #[error("Unknown error: {0}")]
    Other(#[from] anyhow::Error), 
}