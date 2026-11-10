use crate::core::container::DIContainer;
use crate::shared::config::{ConfigError, FragmentConfig};
use crate::shared::FragmentLogger;
use axum::http::Method;
use axum::response::IntoResponse;
use std::path::Path;

pub struct FragmentApp {
    /// The address set by the application code.
    host: Option<String>,
    port: Option<u16>,
    /// The log level set by the application code.
    log_level: Option<String>,
    /// The address read from `fragment.toml`.
    file_host: Option<String>,
    file_port: Option<u16>,
    /// The log level read from `fragment.toml`.
    file_log_level: Option<String>,
    auto_scan: bool,
    compression: bool,
    cors: CorsConfig,
    helmet: bool,
    file_upload: FileUploadConfig,
    controllers: Vec<fn(axum::Router) -> axum::Router>,
}

pub struct CorsConfig {
    pub enabled: bool,
    pub origins: Option<Vec<String>>,
    pub methods: Option<Vec<String>>,
    pub credentials: bool,
}

pub struct FileUploadConfig {
    pub max_size: usize,
    pub allowed_types: Option<Vec<String>>,
    pub max_files: usize,
}

impl FragmentApp {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            log_level: None,
            file_host: None,
            file_port: None,
            file_log_level: None,
            auto_scan: true,
            compression: false,
            cors: CorsConfig {
                enabled: false,
                origins: None,
                methods: None,
                credentials: false,
            },
            helmet: false,
            file_upload: FileUploadConfig {
                max_size: 10 * 1024 * 1024,
                allowed_types: None,
                max_files: 10,
            },
            controllers: Vec::new(),
        }
    }

    /// Build the application from a `fragment.toml` file.
    ///
    /// The file is optional. When it is not there the application starts with
    /// its defaults, so a project runs before it has been configured. When it
    /// is there but cannot be read or parsed, the error is returned rather
    /// than silently ignored.
    ///
    /// ```rust,ignore
    /// use fragment_rs::FragmentApp;
    ///
    /// FragmentApp::from_config_file("fragment.toml")?.start().await;
    /// ```
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }
        Ok(Self::from_config(FragmentConfig::load(path)?))
    }

    /// Build the application from a `fragment.toml` file, ignoring any
    /// problem with the file.
    ///
    /// Use this when a missing or broken configuration file should not stop
    /// the application from starting.
    pub fn from_config_file_or_default(path: impl AsRef<Path>) -> Self {
        Self::from_config_file(path).unwrap_or_default()
    }

    /// Apply a parsed configuration.
    ///
    /// Only the settings the application acts on are applied: the server
    /// address and the log level. Everything else in the file is used by the
    /// components that own it.
    pub fn from_config(config: FragmentConfig) -> Self {
        let mut app = Self::new();
        if let Some(server) = &config.server {
            app.file_host = server.host.clone();
            app.file_port = server.port;
        }
        if let Some(logging) = &config.logging {
            app.file_log_level = logging.level.clone();
        }
        app
    }

    /// The address the server binds to.
    ///
    /// An explicit `host` or `port` wins, then the `FRAGMENT_HOST` and
    /// `FRAGMENT_PORT` environment variables, then the values from
    /// `fragment.toml`, then the defaults.
    pub fn address(&self) -> (String, u16) {
        let host = self
            .host
            .clone()
            .filter(|host| !host.trim().is_empty())
            .or_else(|| {
                std::env::var("FRAGMENT_HOST")
                    .ok()
                    .filter(|host| !host.trim().is_empty())
            })
            .or_else(|| {
                self.file_host
                    .clone()
                    .filter(|host| !host.trim().is_empty())
            })
            .unwrap_or_else(|| "0.0.0.0".to_owned());
        let port = self
            .port
            .or_else(|| {
                std::env::var("FRAGMENT_PORT")
                    .ok()
                    .and_then(|port| port.trim().parse().ok())
            })
            .or(self.file_port)
            .unwrap_or(3000);
        (host, port)
    }

    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the log level used when `RUST_LOG` is not set.
    pub fn log_level(mut self, level: &str) -> Self {
        self.log_level = Some(level.into());
        self
    }

    /// The log level the application was configured with.
    ///
    /// An explicit level wins, then `FRAGMENT_LOG_LEVEL`, then the one from
    /// `fragment.toml`. The `RUST_LOG` environment variable still wins over
    /// all of them, because that is what an operator reaches for.
    pub fn configured_log_level(&self) -> Option<String> {
        self.log_level
            .clone()
            .or_else(|| {
                std::env::var("FRAGMENT_LOG_LEVEL")
                    .ok()
                    .filter(|level| !level.trim().is_empty())
            })
            .or_else(|| self.file_log_level.clone())
    }

    pub fn auto_scan(mut self, enabled: bool) -> Self {
        self.auto_scan = enabled;
        self
    }

    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    pub fn cors_origins(mut self, origins: Vec<String>) -> Self {
        self.cors.enabled = true;
        self.cors.origins = Some(origins);
        self
    }

    pub fn cors_credentials(mut self, enabled: bool) -> Self {
        self.cors.credentials = enabled;
        self
    }

    pub fn helmet(mut self, enabled: bool) -> Self {
        self.helmet = enabled;
        self
    }

    pub fn file_upload_max_size(mut self, size: usize) -> Self {
        self.file_upload.max_size = size;
        self
    }

    pub fn file_upload_allowed_types(mut self, types: Vec<String>) -> Self {
        self.file_upload.allowed_types = Some(types);
        self
    }

    pub fn file_upload_max_files(mut self, max: usize) -> Self {
        self.file_upload.max_files = max;
        self
    }

    /// Registers a service with the dependency injection container.
    ///
    /// The container keeps one instance and hands out an `Arc` of it, so the
    /// service behaves as a singleton.
    ///
    /// This is optional. A `#[service]` type that derives `Default` registers
    /// itself, and the application finds it. Use this for a service the
    /// framework cannot build on its own.
    ///
    /// ```rust,ignore
    /// use fragment_rs::FragmentApp;
    ///
    /// // `CustomerRepository` is a `#[service]` type that needs a
    /// // connection of its own, so it is registered by hand.
    /// FragmentApp::new()
    ///     .service(CustomerRepository::open_pool())
    ///     .start()
    ///     .await;
    /// ```
    pub fn service<T: crate::core::container::Service>(self, instance: T) -> Self {
        DIContainer::register(instance);
        self
    }

    /// Registers a service with a factory, so it is built on first use.
    pub fn service_factory<T, F>(self, factory: F) -> Self
    where
        T: crate::core::container::Service,
        F: Fn() -> T + Send + Sync + 'static,
    {
        DIContainer::register_factory(factory);
        self
    }

    /// Registers a controller's routes with the application.
    ///
    /// This is optional. A `#[controller]` type registers itself, and the
    /// application finds every controller that did. Use this when you want to
    /// be explicit about the order the routes are added, or when a controller
    /// lives in another crate.
    ///
    /// The `#[controller]` macro generates a `register` function on the
    /// controller type, so this is usually just the name of the controller.
    pub fn controller(mut self, register: fn(axum::Router) -> axum::Router) -> Self {
        self.controllers.push(register);
        self
    }

    pub async fn start(self) {
        FragmentLogger::init_with_level(self.configured_log_level().as_deref());
        DIContainer::auto_register_services();

        // Plugins get a chance to set themselves up before the first
        // request arrives.
        crate::plugins::PluginManager::init_all().await;

        let mut router = axum::Router::new();

        if self.cors.enabled {
            use axum::http::HeaderValue;
            use tower_http::cors::{AllowCredentials, Any, CorsLayer};
            let mut layer = CorsLayer::new();
            // Tower rejects the wildcard in an origin list, so it has to be
            // requested as "any" instead.
            let wildcard = self
                .cors
                .origins
                .as_ref()
                .is_some_and(|origins| origins.iter().any(|origin| origin.trim() == "*"));
            if wildcard {
                layer = layer.allow_origin(Any);
            } else if let Some(origins) = &self.cors.origins {
                let origins: Vec<HeaderValue> = origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect();
                if !origins.is_empty() {
                    layer = layer.allow_origin(origins);
                }
            } else {
                layer = layer.allow_origin(Any);
            }
            if let Some(methods) = &self.cors.methods {
                let parsed: Vec<Method> = methods.iter().filter_map(|m| m.parse().ok()).collect();
                layer = layer.allow_methods(parsed);
            }
            if self.cors.credentials {
                layer = layer.allow_credentials(AllowCredentials::yes());
            }
            router = router.layer(layer);
        }

        for ctrl in DIContainer::auto_register_controllers() {
            router = ctrl.register_routes(router);
        }

        // Controllers that registered themselves with the framework.
        for register in crate::core::routing::registered_controllers() {
            router = register(router);
        }

        for register in &self.controllers {
            router = register(router);
        }

        // A route that does not exist answers with the same error envelope
        // as every other error, rather than axum's plain-text default.
        router = router.fallback(not_found);

        let (host, port) = self.address();
        let addr = format!("{host}:{port}");
        tracing::info!("listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        axum::serve(listener, router).await.expect("server error");
    }
}

/// The response for a route that does not exist.
async fn not_found(uri: axum::http::Uri) -> axum::response::Response {
    crate::shared::FragmentError::NotFound(format!("No route for {}", uri.path())).into_response()
}

impl Default for FragmentApp {
    fn default() -> Self {
        Self::new()
    }
}
