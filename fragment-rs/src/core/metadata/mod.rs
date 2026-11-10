use crate::types::{ClassMetadata, MethodMetadata, ParamMetadata, PropertyInjectionMetadata};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct MetadataStorage {
    classes: RwLock<HashMap<String, ClassMetadata>>,
    methods: RwLock<HashMap<String, MethodMetadata>>,
    params: RwLock<HashMap<String, Vec<ParamMetadata>>>,
    property_injections: RwLock<HashMap<String, Vec<PropertyInjectionMetadata>>>,
}

static STORAGE: Lazy<MetadataStorage> = Lazy::new(MetadataStorage::default);

impl MetadataStorage {
    pub fn global() -> &'static MetadataStorage {
        &STORAGE
    }

    pub fn add_class(&self, metadata: ClassMetadata) {
        let key = metadata.target.clone();
        self.classes.write().unwrap().insert(key, metadata);
    }

    pub fn get_class(&self, target: &str) -> Option<ClassMetadata> {
        self.classes.read().unwrap().get(target).cloned()
    }

    pub fn get_all_classes(&self) -> Vec<ClassMetadata> {
        self.classes.read().unwrap().values().cloned().collect()
    }

    pub fn add_method(&self, metadata: MethodMetadata) {
        let key = format!("{}::{}", metadata.target, metadata.property_key);
        self.methods.write().unwrap().insert(key, metadata);
    }

    pub fn get_method(&self, target: &str, property_key: &str) -> Option<MethodMetadata> {
        let key = format!("{}::{}", target, property_key);
        self.methods.read().unwrap().get(&key).cloned()
    }

    pub fn get_all_methods(&self) -> Vec<MethodMetadata> {
        self.methods.read().unwrap().values().cloned().collect()
    }

    pub fn get_controller_methods(&self, target: &str) -> Vec<MethodMetadata> {
        self.get_all_methods()
            .into_iter()
            .filter(|m| m.target == target)
            .collect()
    }

    pub fn add_param(&self, metadata: ParamMetadata) {
        let key = format!("{}::{}", metadata.target, metadata.property_key);
        let mut params = self.params.write().unwrap();
        params.entry(key).or_default().push(metadata);
    }

    pub fn get_params(&self, target: &str, property_key: &str) -> Vec<ParamMetadata> {
        let key = format!("{}::{}", target, property_key);
        self.params
            .read()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_property_injection(&self, target: &str, injection: PropertyInjectionMetadata) {
        let mut injections = self.property_injections.write().unwrap();
        injections
            .entry(target.to_string())
            .or_default()
            .push(injection);
    }

    pub fn get_property_injections(&self, target: &str) -> Vec<PropertyInjectionMetadata> {
        self.property_injections
            .read()
            .unwrap()
            .get(target)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        self.classes.write().unwrap().clear();
        self.methods.write().unwrap().clear();
        self.params.write().unwrap().clear();
        self.property_injections.write().unwrap().clear();
    }
}

impl Default for MetadataStorage {
    fn default() -> Self {
        Self {
            classes: RwLock::new(HashMap::new()),
            methods: RwLock::new(HashMap::new()),
            params: RwLock::new(HashMap::new()),
            property_injections: RwLock::new(HashMap::new()),
        }
    }
}

pub fn storage() -> &'static MetadataStorage {
    MetadataStorage::global()
}
