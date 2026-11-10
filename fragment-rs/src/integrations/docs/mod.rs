use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiRoute {
    pub method: String,
    pub path: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: ApiInfo,
    pub paths: HashMap<String, HashMap<String, PathItem>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

impl Default for ApiInfo {
    fn default() -> Self {
        Self {
            title: "Fragment API".to_string(),
            version: "1.0.0".to_string(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PathItem {
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub responses: HashMap<String, ResponseItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseItem {
    pub description: String,
}

pub struct OpenApiBuilder {
    spec: OpenApiSpec,
}

impl OpenApiBuilder {
    pub fn new() -> Self {
        Self {
            spec: OpenApiSpec {
                openapi: "3.1.0".to_string(),
                info: ApiInfo::default(),
                paths: HashMap::new(),
            },
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.spec.info.title = title.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.spec.info.version = version.into();
        self
    }

    pub fn add_route(mut self, method: &str, path: &str, summary: &str, tags: Vec<String>) -> Self {
        self.spec.paths.entry(path.to_string()).or_default().insert(
            method.to_lowercase(),
            PathItem {
                summary: Some(summary.to_string()),
                tags,
                responses: HashMap::from([(
                    "200".to_string(),
                    ResponseItem {
                        description: "Success".to_string(),
                    },
                )]),
            },
        );
        self
    }

    pub fn build(self) -> OpenApiSpec {
        self.spec
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.spec).unwrap_or_default()
    }
}

impl Default for OpenApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}
