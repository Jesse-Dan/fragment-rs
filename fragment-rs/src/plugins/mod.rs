use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn on_init(self: Arc<Self>) -> Pin<Box<dyn Future<Output = ()> + Send>>
    where
        Self: Send,
    {
        Box::pin(async {})
    }
    fn on_shutdown(self: Arc<Self>) -> Pin<Box<dyn Future<Output = ()> + Send>>
    where
        Self: Send,
    {
        Box::pin(async {})
    }
}

pub struct PluginManager;

impl PluginManager {
    fn plugins() -> std::sync::MutexGuard<'static, Vec<Arc<dyn Plugin>>> {
        static PLUGINS: once_cell::sync::Lazy<std::sync::Mutex<Vec<Arc<dyn Plugin>>>> =
            once_cell::sync::Lazy::new(|| std::sync::Mutex::new(Vec::new()));
        PLUGINS.lock().unwrap()
    }

    pub fn register<P: Plugin + 'static>(plugin: Arc<P>) {
        Self::plugins().push(plugin);
    }

    pub fn get_all() -> Vec<Arc<dyn Plugin>> {
        PluginManager::plugins().iter().cloned().collect()
    }

    pub fn find_by_name(name: &str) -> Option<Arc<dyn Plugin>> {
        PluginManager::plugins()
            .iter()
            .find(|p| p.name() == name)
            .cloned()
    }

    pub async fn init_all() {
        for plugin in PluginManager::plugins().iter() {
            plugin.clone().on_init().await;
        }
    }

    pub async fn shutdown_all() {
        for plugin in PluginManager::plugins().iter() {
            plugin.clone().on_shutdown().await;
        }
    }

    pub fn clear() {
        PluginManager::plugins().clear();
    }
}
