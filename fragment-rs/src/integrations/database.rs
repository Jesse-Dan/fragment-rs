use crate::core::container::Service;
use crate::shared::config::DatabaseConfig;
use sqlx::any::{install_default_drivers, AnyPoolOptions};
use sqlx::AnyPool;
use std::collections::HashMap;
use std::sync::{Arc, Once, RwLock};

static INSTALL_DRIVERS: Once = Once::new();

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database configuration `{0}` is missing a connection URL")]
    MissingUrl(String),
    #[error("Database connection `{0}` is not configured")]
    NotFound(String),
    #[error("Failed to connect to database `{0}`")]
    Connection(String),
}

impl axum::response::IntoResponse for DatabaseError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        tracing::error!(error = %self, "database operation failed");
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

#[derive(Clone, Default)]
pub struct DatabaseManager {
    pools: Arc<RwLock<HashMap<String, AnyPool>>>,
}

impl Service for DatabaseManager {}

impl DatabaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Establish and register one named database pool.
    pub async fn connect(
        &self,
        name: impl Into<String>,
        config: &DatabaseConfig,
    ) -> Result<(), DatabaseError> {
        let name = name.into();
        let url = config
            .url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| DatabaseError::MissingUrl(name.clone()))?;

        INSTALL_DRIVERS.call_once(install_default_drivers);
        let max_connections = config.pool_size.unwrap_or(10).max(1);
        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await
            .map_err(|_| {
                tracing::error!(database = %name, "database connection failed");
                DatabaseError::Connection(name.clone())
            })?;

        let previous = self.pools.write().unwrap().insert(name, pool);
        if let Some(previous) = previous {
            previous.close().await;
        }
        Ok(())
    }

    /// Connect each entry and register it under its configuration key.
    pub async fn connect_all(
        &self,
        databases: &HashMap<String, DatabaseConfig>,
    ) -> Result<(), DatabaseError> {
        for (name, config) in databases {
            if let Err(error) = self.connect(name.clone(), config).await {
                self.close_all().await;
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn pool(&self, name: &str) -> Result<AnyPool, DatabaseError> {
        self.pools
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| DatabaseError::NotFound(name.to_owned()))
    }

    pub async fn close_all(&self) {
        let pools = std::mem::take(&mut *self.pools.write().unwrap());
        for (_, pool) in pools {
            pool.close().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatabaseError, DatabaseManager};
    use crate::shared::config::DatabaseConfig;
    use sqlx::Row;

    #[tokio::test]
    async fn registers_and_queries_named_sqlite_pool() {
        let manager = DatabaseManager::new();
        let config = DatabaseConfig {
            url: Some("sqlite::memory:".into()),
            pool_size: Some(1),
        };
        manager.connect("primary", &config).await.unwrap();

        let pool = manager.pool("primary").unwrap();
        let row = sqlx::query("SELECT 42 AS answer")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<i64, _>("answer"), 42);
        assert!(matches!(
            manager.pool("missing"),
            Err(DatabaseError::NotFound(_))
        ));
        manager.close_all().await;
    }

    #[tokio::test]
    async fn supports_isolated_named_connections() {
        let manager = DatabaseManager::new();
        let config = DatabaseConfig {
            url: Some("sqlite::memory:".into()),
            pool_size: Some(1),
        };
        manager.connect("primary", &config).await.unwrap();
        manager.connect("analytics", &config).await.unwrap();

        let primary = manager.pool("primary").unwrap();
        let analytics = manager.pool("analytics").unwrap();
        sqlx::query("CREATE TABLE sample (value INTEGER)")
            .execute(&primary)
            .await
            .unwrap();
        assert!(sqlx::query("SELECT * FROM sample")
            .fetch_one(&analytics)
            .await
            .is_err());
        manager.close_all().await;
    }

    #[tokio::test]
    async fn reports_missing_connection_url() {
        let manager = DatabaseManager::new();
        let config = DatabaseConfig {
            url: None,
            pool_size: None,
        };
        assert!(matches!(
            manager.connect("analytics", &config).await,
            Err(DatabaseError::MissingUrl(name)) if name == "analytics"
        ));
    }
}
