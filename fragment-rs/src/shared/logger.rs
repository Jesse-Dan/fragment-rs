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
        Self::init_with_level(None);
    }

    /// Initialise logging with a fallback level.
    ///
    /// `RUST_LOG` still wins, so an operator can turn up the verbosity
    /// without editing the configuration file. The fallback is used when
    /// `RUST_LOG` is not set.
    pub fn init_with_level(level: Option<&str>) {
        INITIALIZED.get_or_init(|| {
            let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                level
                    .and_then(|level| EnvFilter::try_new(level).ok())
                    .unwrap_or_else(|| EnvFilter::new("info"))
            });
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_span_events(FmtSpan::CLOSE)
                .init();
        });
    }
}
