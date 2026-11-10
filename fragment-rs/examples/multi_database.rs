use fragment_rs::{DatabaseConfig, DatabaseManager};
use sqlx::Row;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut databases = HashMap::new();
    databases.insert(
        "primary".to_owned(),
        DatabaseConfig {
            url: Some("sqlite://fragment-primary.db?mode=rwc".to_owned()),
            pool_size: Some(5),
        },
    );
    databases.insert(
        "analytics".to_owned(),
        DatabaseConfig {
            url: Some("sqlite://fragment-analytics.db?mode=rwc".to_owned()),
            pool_size: Some(2),
        },
    );

    let manager = DatabaseManager::new();
    manager.connect_all(&databases).await?;

    let primary = manager.pool("primary")?;
    let row = sqlx::query("SELECT 1 AS healthy")
        .fetch_one(&primary)
        .await?;
    println!(
        "primary database healthy: {}",
        row.get::<i64, _>("healthy") == 1
    );

    manager.close_all().await;
    Ok(())
}
