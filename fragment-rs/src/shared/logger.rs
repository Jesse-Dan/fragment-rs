use std::sync::OnceLock;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

static INITIALIZED: OnceLock<()> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Silent,
    Error,
    Warn,
    Info,
    Debug,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "silent" => Self::Silent,
            "error" => Self::Error,
            "warn" | "warning" => Self::Warn,
            "info" => Self::Info,
            "debug" | "trace" => Self::Debug,
            _ => Self::Info,
        }
    }
}

pub struct FragmentLogger;

impl FragmentLogger {
    pub fn init() {
        INITIALIZED.get_or_init(|| {
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_span_events(FmtSpan::CLOSE)
                .init();
        });
    }
}
