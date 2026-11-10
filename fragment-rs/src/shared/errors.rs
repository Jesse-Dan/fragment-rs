use thiserror::Error;

#[derive(Debug, Error)]
pub enum FragmentError {
    #[error("{message}")]
    Generic {
        message: String,
        status_code: u16,
        code: String,
        details: Option<serde_json::Value>,
    },

    #[error("Service not registered: {0}")]
    DependencyResolution(#[from] DependencyResolution),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Request timeout")]
    RequestTimeout,

    #[error("Rate limit exceeded")]
    RateLimitExceeded { retry_after: u64 },
}

impl FragmentError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Generic { status_code, .. } => *status_code,
            Self::DependencyResolution(_) => 500,
            Self::Validation(_) => 422,
            Self::Database(_) => 500,
            Self::Configuration(_) => 500,
            Self::NotFound(_) => 404,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::RequestTimeout => 408,
            Self::RateLimitExceeded { .. } => 429,
        }
    }

    pub fn code(&self) -> String {
        match self {
            Self::Generic { code, .. } => code.clone(),
            Self::DependencyResolution(_) => "DEPENDENCY_RESOLUTION".into(),
            Self::Validation(_) => "VALIDATION_ERROR".into(),
            Self::Database(_) => "DATABASE_ERROR".into(),
            Self::Configuration(_) => "CONFIGURATION_ERROR".into(),
            Self::NotFound(_) => "NOT_FOUND".into(),
            Self::Unauthorized => "UNAUTHORIZED".into(),
            Self::Forbidden => "FORBIDDEN".into(),
            Self::RequestTimeout => "REQUEST_TIMEOUT".into(),
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED".into(),
        }
    }

    pub fn new(message: impl Into<String>, status_code: u16) -> Self {
        Self::Generic {
            message: message.into(),
            status_code,
            code: format!(
                "E{}",
                status_code
            ),
            details: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum DependencyResolution {
    #[error("Circular dependency: {0}")]
    Circular(String),

    #[error("Not registered: {0}")]
    NotFound(String),

    #[error("Multiple implementations for {0}")]
    Ambiguous(String),
}