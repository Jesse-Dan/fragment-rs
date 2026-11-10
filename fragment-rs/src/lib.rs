pub mod core;
pub mod integrations;
pub mod platform;
pub mod plugins;
pub mod shared;
pub mod types;

pub use fragment_macros::*;

pub use crate::core::container::{DIContainer, Inject, Service, register_service, register_controller};
pub use crate::core::filter::ExceptionFilter;
pub use crate::core::http::FrgClient;
pub use crate::core::privacy::Privacy;
pub use crate::core::routing::{FragmentRoute, FragmentRouter, GET, POST, PUT, DELETE, PATCH};
pub use crate::core::validation::{ValidatedJson, ValidatedQuery};
pub use crate::core::workflow::{WorkflowDef, WorkflowRunner};
pub use crate::platform::web::{FragmentApp, FragmentBootstrap, CorsConfig, FileUploadConfig};
pub use crate::shared::{FragmentError, FragmentLogger};

pub use validator::Validate;
