use serde::Deserialize;
use std::path::Path;

// toml is available via the crate-level dependency

#[derive(Debug, Default, Deserialize)]
pub struct FragmentConfig {
    pub server: Option<ServerConfig>,
    pub database: Option<DatabaseConfig>,
    pub logging: Option<LoggingConfig>,
    pub cache: Option<CacheConfig>,
    pub rate_limit: Option<RateLimitConfig>,
    pub privacy: Option<PrivacyConfig>,
    pub workflows: Option<WorkflowsConfig>,
    pub docs: Option<DocsConfig>,
    pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: Option<String>,
    pub pool_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: Option<String>,
    pub format: Option<String>,
    pub structured: Option<bool>,
    pub file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    pub driver: Option<String>,
    pub prefix: Option<String>,
    pub ttl: Option<u64>,
    pub redis: Option<RedisConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    pub driver: Option<String>,
    pub prefix: Option<String>,
    pub requests: Option<u64>,
    pub window: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PrivacyConfig {
    pub redact_logs: Option<bool>,
    pub encryption_key: Option<String>,
    pub mask: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowsConfig {
    pub storage: Option<String>,
    pub local_path: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DocsConfig {
    pub output_dir: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
    pub path: Option<String>,
    pub enabled: Option<bool>,
}

impl FragmentConfig {
    /// Load from a TOML file. Requires `toml` crate as dependency.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        // Placeholder - requires toml::from_str import
        let _content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::Io(e.to_string()))?;
        // TODO: Add `toml` dependency and use: toml::from_str(&content)?
        Ok(FragmentConfig::default())
    }

    pub fn load_defaults() -> FragmentConfig {
        FragmentConfig::default()
    }

    pub fn from_env() -> Self {
        Self {
            server: Some(ServerConfig {
                host: std::env::var("FRAGMENT_HOST").ok(),
                port: std::env::var("FRAGMENT_PORT").ok().and_then(|s| s.parse().ok()),
                workers: None,
            }),
            logging: Some(LoggingConfig {
                level: std::env::var("FRAGMENT_LOG_LEVEL").ok(),
                format: None,
                structured: std::env::var("FRAGMENT_LOG_STRUCTURED").ok().map(|s| s == "true"),
                file: None,
            }),
            verbose: std::env::var("FRAGMENT_VERBOSE").ok().map(|s| s == "true"),
            ..Default::default()
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
}