use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassType {
    Service,
    Controller,
    Component,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMetadata {
    pub target: String,
    pub class_type: ClassType,
    pub scope: Scope,
    pub path: Option<String>,
    pub qualifier: Option<String>,
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodMetadata {
    pub target: String,
    pub property_key: String,
    pub http_method: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType {
    Body,
    Param,
    Query,
    Header,
    Context,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamMetadata {
    pub target: String,
    pub property_key: String,
    pub param_type: ParamType,
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionType {
    Constructor,
    Property,
    Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInjectionMetadata {
    pub target: String,
    pub property_key: String,
    pub injection_type: InjectionType,
    pub type_name: String,
}
