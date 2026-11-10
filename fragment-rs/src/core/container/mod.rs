use crate::core::metadata::storage;
use crate::core::routing::FragmentRoute;
use crate::types::{ClassMetadata, ClassType, Scope as TypeScope};
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

static CONTAINER: LazyLock<Arc<ContainerInner>> = LazyLock::new(|| Arc::new(ContainerInner::default()));

static SERVICE_REGISTRY: LazyLock<RwLock<Vec<fn()>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

static CONTROLLER_REGISTRY: LazyLock<RwLock<Vec<Box<dyn Fn() -> Arc<dyn FragmentRoute + Send + Sync> + Send + Sync>>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

struct ContainerInner {
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    factories: RwLock<HashMap<TypeId, Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>>>,
    registered: RwLock<HashMap<TypeId, bool>>,
    constructing: RwLock<HashMap<TypeId, bool>>,
    scoped: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl Default for ContainerInner {
    fn default() -> Self {
        Self {
            singletons: RwLock::new(HashMap::new()),
            factories: RwLock::new(HashMap::new()),
            registered: RwLock::new(HashMap::new()),
            constructing: RwLock::new(HashMap::new()),
            scoped: RwLock::new(HashMap::new()),
        }
    }
}

fn container() -> Arc<ContainerInner> {
    CONTAINER.clone()
}

impl ContainerInner {
    fn register<T: Service>(&self, instance: T) where T: 'static {
        let tid = TypeId::of::<T>();
        self.registered.write().unwrap().insert(tid, true);
        self.singletons.write().unwrap().insert(tid, Arc::new(instance));
    }

    fn register_factory<T: Service, F>(&self, factory: F)
    where
        T: 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        let tid = TypeId::of::<T>();
        self.registered.write().unwrap().insert(tid, true);
        self.factories.write().unwrap().insert(
            tid,
            Box::new(move || Arc::new(factory()) as Arc<dyn Any + Send + Sync>) as Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>
        );
    }

    fn get<T: Service>(&self) -> Option<Arc<T>> where T: 'static {
        let tid = TypeId::of::<T>();

        if let Some(s) = self.singletons.read().unwrap().get(&tid) {
            if let Ok(casted) = s.clone().downcast::<T>() {
                return Some(casted);
            }
        }

        if let Some(f) = self.factories.read().unwrap().get(&tid) {
            let instance = (f)();
            if let Ok(casted) = instance.downcast::<T>() {
                return Some(casted);
            }
        }

        None
    }

    fn has<T: Service>(&self) -> bool {
        let tid = TypeId::of::<T>();
        self.registered.read().unwrap().contains_key(&tid)
            || self.singletons.read().unwrap().contains_key(&tid)
            || self.factories.read().unwrap().contains_key(&tid)
    }

    fn clear(&self) {
        self.singletons.write().unwrap().clear();
        self.factories.write().unwrap().clear();
        self.registered.write().unwrap().clear();
        self.constructing.write().unwrap().clear();
        self.scoped.write().unwrap().clear();
    }
}

pub trait Service: Clone + Send + Sync + 'static {}

pub trait Injectable: Service {
    fn scope() -> Scope {
        Scope::Singleton
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Singleton,
    Request,
    Transient,
}

impl Default for Scope {
    fn default() -> Self {
        Self::Singleton
    }
}

pub struct DIContainer;

impl DIContainer {
    pub fn register<T: Service>(instance: T) {
        container().register(instance);
        storage().add_class(ClassMetadata {
            target: std::any::type_name::<T>().to_string(),
            class_type: ClassType::Service,
            scope: TypeScope::Singleton,
            path: None,
            qualifier: None,
            provides: vec![],
        });
    }

    pub fn register_factory<T: Service, F>(factory: F)
    where
        T: 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        container().register_factory(factory);
    }

    pub fn get<T: Service>() -> Option<Arc<T>> {
        container().get()
    }

    pub fn has<T: Service>() -> bool {
        container().has::<T>()
    }

    pub fn clear() {
        container().clear();
    }

    pub fn auto_register_services() {
        for callback in SERVICE_REGISTRY.read().unwrap().iter() {
            callback();
        }
    }

    pub fn auto_register_controllers() -> Vec<Arc<dyn FragmentRoute + Send + Sync>> {
        CONTROLLER_REGISTRY
            .read()
            .unwrap()
            .iter()
            .map(|f| (f)())
            .collect()
    }
}

pub fn register_service(callback: fn()) {
    SERVICE_REGISTRY.write().unwrap().push(callback);
}

pub fn register_controller<F>(factory: F)
where
    F: Fn() -> Arc<dyn FragmentRoute + Send + Sync> + Send + Sync + 'static,
{
    CONTROLLER_REGISTRY.write().unwrap().push(Box::new(factory) as Box<dyn Fn() -> Arc<dyn FragmentRoute + Send + Sync> + Send + Sync>);
}

pub struct Inject<T: Service>(pub Arc<T>);

impl<T: Service> Clone for Inject<T> {
    fn clone(&self) -> Self {
        Inject(self.0.clone())
    }
}

impl<T: Service> std::ops::Deref for Inject<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: Service,
    S: Send + Sync,
{
    type Rejection = DiRejection;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        DIContainer::get::<T>()
            .map(Inject)
            .ok_or_else(|| DiRejection::NotFound(std::any::type_name::<T>().to_string()))
    }
}

#[derive(Debug)]
pub enum DiRejection {
    NotFound(String),
    Circular(String),
}

impl std::fmt::Display for DiRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "Service not registered: {}", s),
            Self::Circular(s) => write!(f, "Circular dependency: {}", s),
        }
    }
}

impl IntoResponse for DiRejection {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            DiRejection::NotFound(s) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Service `{}` not registered in DI container", s)),
            DiRejection::Circular(s) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Circular dependency: {}", s)),
        };
        (status, Json(serde_json::json!({
            "error": msg,
            "code": "DI_ERROR"
        }))).into_response()
    }
}
