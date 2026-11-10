use axum::Router;

/// The inventory registry that collects the controllers of an application.
pub use inventory;

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

/// A controller that registered itself with the framework.
///
/// The `#[controller]` macro submits one of these for every controller, so
/// the application can find all of them without being told about them.
pub struct ControllerRegistration {
    /// Registers the controller's routes on a router.
    pub register: fn(Router) -> Router,
}

inventory::collect!(ControllerRegistration);

/// Every controller that registered itself with the framework.
pub fn registered_controllers() -> Vec<fn(Router) -> Router> {
    inventory::iter::<ControllerRegistration>()
        .map(|registration| registration.register)
        .collect()
}

pub struct FragmentRouter {
    host: String,
    port: u16,
    controllers: Vec<fn(axum::Router) -> axum::Router>,
}

impl FragmentRouter {
    pub fn new() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
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

    /// Registers a controller's routes with the application.
    ///
    /// The `#[controller]` macro generates a `register` function on the
    /// controller type, so this is usually just the name of the controller.
    pub fn controller(mut self, register: fn(axum::Router) -> axum::Router) -> Self {
        self.controllers.push(register);
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

        // Controllers that registered themselves with the framework.
        for register in registered_controllers() {
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

impl Default for FragmentRouter {
    fn default() -> Self {
        Self::new()
    }
}
