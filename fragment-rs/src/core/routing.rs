use axum::Router;

pub const GET: &str = "GET";
pub const POST: &str = "POST";
pub const PUT: &str = "PUT";
pub const DELETE: &str = "DELETE";
pub const PATCH: &str = "PATCH";

pub trait FragmentRoute: Send + Sync + 'static {
    fn register_routes(&self, router: Router) -> Router {
        router
    }
}

pub trait Controller: Send + Sync + 'static {
    fn register_routes(router: Router) -> Router;
}

impl<T: Controller> FragmentRoute for T {
    fn register_routes(&self, router: Router) -> Router {
        T::register_routes(router)
    }
}

pub struct FragmentRouter {
    host: String,
    port: u16,
}

impl FragmentRouter {
    pub fn new() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
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

    pub async fn start(self) {
        use crate::shared::FragmentLogger;
        FragmentLogger::init();
        crate::core::container::DIContainer::auto_register_services();

        let mut router = Router::new();

        for ctrl in crate::core::container::DIContainer::auto_register_controllers() {
            router = ctrl.register_routes(router);
        }

        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!("listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        axum::serve(listener, router).await.expect("server error");
    }
}

impl Default for FragmentRouter {
    fn default() -> Self {
        Self::new()
    }
}
