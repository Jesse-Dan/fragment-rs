use fragment_rs::{
    orm::{entity::prelude::*, ConnectionTrait, DatabaseBackend, Schema, Set},
    DatabaseConfig, OrmDatabaseManager,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let databases = OrmDatabaseManager::new();
    databases
        .connect(
            "primary",
            &DatabaseConfig {
                url: Some("sqlite::memory:".to_owned()),
                pool_size: Some(1),
            },
        )
        .await?;
    let connection = databases.connection("primary")?;

    let schema = Schema::new(DatabaseBackend::Sqlite);
    let create_table = schema.create_table_from_entity(Entity);
    connection
        .execute(connection.get_database_backend().build(&create_table))
        .await?;

    let note = ActiveModel {
        title: Set("SeaORM entity example".to_owned()),
        ..Default::default()
    }
    .insert(&connection)
    .await?;

    let loaded = Entity::find_by_id(note.id)
        .one(&connection)
        .await?
        .expect("inserted note should exist");
    println!("Loaded note: {}", loaded.title);

    databases.close_all().await;
    Ok(())
}
