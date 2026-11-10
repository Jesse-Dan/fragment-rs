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

    pub async fn start(self) {
        FragmentLogger::init();
        DIContainer::auto_register_services();

        let mut router = axum::Router::new();

        if self.cors.enabled {
            use tower_http::cors::{CorsLayer, Any, AllowCredentials};
            use axum::http::HeaderValue;
            let mut layer = CorsLayer::new();
            if let Some(origins) = &self.cors.origins {
                let origins: Vec<HeaderValue> = origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
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

        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!("listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        axum::serve(listener, router)
            .await
            .expect("server error");
    }
}

impl Default for FragmentApp {
    fn default() -> Self {
        Self::new()
    }
}
