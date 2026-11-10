use std::collections::HashMap;

pub struct FragmentBootstrap {
    components: HashMap<String, ComponentInfo>,
}

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub component_type: String,
}

impl FragmentBootstrap {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn boot(app: &str, port: u16, components: usize) {
        tracing::info!(
            "{} v{} starting on http://0.0.0.0:{}",
            app,
            env!("CARGO_PKG_VERSION"),
            port
        );
        tracing::info!("{} components registered", components);
    }
}

impl Default for FragmentBootstrap {
    fn default() -> Self {
        Self::new()
    }
}
