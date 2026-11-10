use crate::core::container::DIContainer;
use crate::shared::FragmentLogger;
use axum::http::Method;

pub struct FragmentApp {
    host: String,
    port: u16,
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
            host: "0.0.0.0".into(),
            port: 3000,
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

    pub fn host(mut self, host: &str) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
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
        FragmentLogger::init();
        DIContainer::auto_register_services();

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

        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!("listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        axum::serve(listener, router).await.expect("server error");
    }
}

impl Default for FragmentApp {
    fn default() -> Self {
        Self::new()
    }
}
