use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub enum Scope {
    Request,
    Application,
}

impl Default for Scope {
    fn default() -> Self {
        Self::Application
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl RequestContext {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            user_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

static STORAGE: Lazy<RwLock<Option<RequestContext>>> = Lazy::new(|| RwLock::new(None));

pub struct RequestContextStorage;

impl RequestContextStorage {
    pub fn set(ctx: RequestContext) {
        *STORAGE.write().unwrap() = Some(ctx);
    }

    pub fn get() -> Option<RequestContext> {
        STORAGE.read().unwrap().clone()
    }

    pub fn clear() {
        *STORAGE.write().unwrap() = None;
    }

    pub fn with<T>(ctx: RequestContext, f: impl FnOnce() -> T) -> T {
        Self::set(ctx);
        let result = f();
        Self::clear();
        result
    }
}

pub fn current_context() -> Option<RequestContext> {
    RequestContextStorage::get()
}

pub fn request_id() -> Option<String> {
    current_context().map(|c| c.request_id)
}