use crate::core::container::Service;
use crate::shared::config::DatabaseConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, thiserror::Error)]
pub enum OrmError {
    #[error("Database configuration `{0}` is missing a connection URL")]
    MissingUrl(String),
    #[error("Database connection `{0}` is not configured")]
    NotFound(String),
    #[error("Failed to connect ORM database `{0}`")]
    Connection(String),
}

impl axum::response::IntoResponse for OrmError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        tracing::error!(error = %self, "ORM database operation failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": {
                    "code": "DATABASE_ERROR",
                    "message": "A database operation failed"
                }
            })),
        )
            .into_response()
    }
}

/// Named SeaORM database connections, complementing `DatabaseManager`'s raw SQLx pools.
#[derive(Clone, Default)]
pub struct OrmDatabaseManager {
    connections: Arc<RwLock<HashMap<String, DatabaseConnection>>>,
}

impl Service for OrmDatabaseManager {}

impl OrmDatabaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(
        &self,
        name: impl Into<String>,
        config: &DatabaseConfig,
    ) -> Result<(), OrmError> {
        let name = name.into();
        let url = config
            .url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| OrmError::MissingUrl(name.clone()))?;
        let mut options = ConnectOptions::new(url.to_owned());
        options.max_connections(config.pool_size.unwrap_or(10).max(1));
        let connection = Database::connect(options).await.map_err(|_| {
            tracing::error!(database = %name, "ORM database connection failed");
            OrmError::Connection(name.clone())
        })?;

        let previous = self.connections.write().unwrap().insert(name, connection);
        if let Some(previous) = previous {
            previous
                .close()
                .await
                .map_err(|_| OrmError::Connection("replaced connection".to_owned()))?;
        }
        Ok(())
    }

    pub async fn connect_all(
        &self,
        databases: &HashMap<String, DatabaseConfig>,
    ) -> Result<(), OrmError> {
        for (name, config) in databases {
            if let Err(error) = self.connect(name.clone(), config).await {
                self.close_all().await;
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn connection(&self, name: &str) -> Result<DatabaseConnection, OrmError> {
        self.connections
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| OrmError::NotFound(name.to_owned()))
    }

    pub async fn close_all(&self) {
        let connections = std::mem::take(&mut *self.connections.write().unwrap());
        for (_, connection) in connections {
            if let Err(error) = connection.close().await {
                tracing::warn!(error = %error, "failed to close ORM database connection");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OrmDatabaseManager;
    use crate::shared::config::DatabaseConfig;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseBackend, EntityTrait, Schema, Set};

    mod test_user {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "orm_users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub name: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    #[tokio::test]
    async fn creates_and_reads_an_entity_using_a_named_connection() {
        let manager = OrmDatabaseManager::new();
        manager
            .connect(
                "primary",
                &DatabaseConfig {
                    url: Some("sqlite::memory:".to_owned()),
                    pool_size: Some(1),
                },
            )
            .await
            .unwrap();
        let db = manager.connection("primary").unwrap();

        let schema = Schema::new(DatabaseBackend::Sqlite);
        let statement = schema.create_table_from_entity(test_user::Entity);
        db.execute(db.get_database_backend().build(&statement))
            .await
            .unwrap();

        let inserted = test_user::ActiveModel {
            name: Set("Ada".to_owned()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let found = test_user::Entity::find_by_id(inserted.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Ada");
        manager.close_all().await;
    }
}
