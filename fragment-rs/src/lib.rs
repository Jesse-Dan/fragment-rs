pub mod core;
pub mod integrations;
pub mod platform;
pub mod plugins;
pub mod shared;
pub mod types;

pub use fragment_macros::*;

pub use crate::core::container::{
    register_controller, register_service, DIContainer, Inject, Service,
};
pub use crate::core::filter::ExceptionFilter;
pub use crate::core::http::FrgClient;
pub use crate::core::privacy::Privacy;
pub use crate::core::routing::{FragmentRoute, FragmentRouter, DELETE, GET, PATCH, POST, PUT};
pub use crate::core::validation::{ValidatedJson, ValidatedQuery};
pub use crate::core::workflow::{WorkflowDef, WorkflowRunner};
pub use crate::integrations::database::{DatabaseError, DatabaseManager};
pub use crate::integrations::orm::{OrmDatabaseManager, OrmError};
pub use crate::platform::web::{CorsConfig, FileUploadConfig, FragmentApp, FragmentBootstrap};
pub use crate::shared::config::{DatabaseConfig, FragmentConfig};
pub use crate::shared::{FragmentError, FragmentLogger};
pub use sea_orm as orm;

pub use validator::Validate;
