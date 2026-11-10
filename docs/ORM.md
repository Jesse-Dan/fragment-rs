# ORM and databases

Fragment Rust ships two database layers:

| Layer | Type | Use it for |
|---|---|---|
| **SeaORM** (re-exported as `fragment_rs::orm`) | `OrmDatabaseManager` | Entities, ActiveModels, relations, typed queries, transactions |
| **SQLx** | `DatabaseManager` | Raw SQL, driver-specific tuning, reporting queries |

Both managers keep **named connections**, so an application can talk to several
databases at once — for example a primary PostgreSQL database and an analytics
SQLite database — and choose the target explicitly per operation.

## Named connections

Configure databases in `fragment.toml`:

```toml
[databases.primary]
url = "postgres://postgres:postgres@localhost/fragment"
pool_size = 10

[databases.analytics]
url = "sqlite://analytics.db?mode=rwc"
pool_size = 2
```

`FragmentConfig` exposes them as a map:

```rust
use fragment_rs::{DatabaseConfig, FragmentConfig};

let config = FragmentConfig::load("fragment.toml")?;
for (name, database) in &config.databases {
    println!("{name}: {:?}", database.url);
}
```

Each `DatabaseConfig` has two fields:

| Field | Type | Default | Description |
|---|---|---|---|
| `url` | `Option<String>` | — | Connection URL. The scheme selects the driver. |
| `pool_size` | `Option<u32>` | `10` | Maximum number of pooled connections. |

Supported URL schemes:

| Scheme | Driver |
|---|---|
| `postgres://` / `postgresql://` | PostgreSQL |
| `mysql://` | MySQL / MariaDB |
| `sqlite:` / `sqlite://` | SQLite |

## SeaORM: entities and queries

SeaORM is re-exported as `fragment_rs::orm`, so entity derives are available
without adding `sea-orm` to your own manifest by hand. The generated
`frs make:entity` command does add it for you, because SeaORM's derive macros
expand to `sea_orm::` paths and therefore need a direct dependency.

Define an entity:

```rust
use fragment_rs::orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub email: String,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

Connect and query:

```rust
use fragment_rs::{OrmDatabaseManager, orm::entity::prelude::*};

let databases = OrmDatabaseManager::new();

databases
    .connect("primary", &DatabaseConfig {
        url: Some("postgres://postgres:postgres@localhost/fragment".into()),
        pool_size: Some(10),
    })
    .await?;

let db = databases.connection("primary")?;

// Insert
let user = user::ActiveModel {
    email: Set("ada@example.com".into()),
    name: Set("Ada".into()),
    ..Default::default()
}
.insert(&db)
.await?;

// Read
let found = user::Entity::find_by_id(user.id).one(&db).await?.unwrap();

// Update
let mut active: user::ActiveModel = found.into();
active.name = Set("Ada Lovelace".into());
active.update(&db).await?;
```

### Manager API

| Method | Purpose |
|---|---|
| `connect(name, &config)` | Open one named connection |
| `connect_all(&databases)` | Open every connection in a config map |
| `connection(name)` | Borrow a `DatabaseConnection` by name |
| `close_all()` | Close every open connection |

`connect_all` closes the connections it opened if any later connection fails,
so a partial failure does not leave pools behind.

### Transactions

Transactions are scoped to a single connection:

```rust
let db = databases.connection("primary")?;
let txn = db.begin().await?;

user::Entity::update_many()
    .col_expr(user::Column::Name, Expr::value("Ada"))
    .exec(&txn)
    .await?;

txn.commit().await?;
```

A transaction cannot span two named connections. Use one connection per
transaction.

### Raw SQL with SQLx

When you need a query SeaORM does not model, use the SQLx manager:

```rust
use fragment_rs::{DatabaseManager, DatabaseConfig};

let databases = DatabaseManager::new();
databases
    .connect("primary", &DatabaseConfig {
        url: Some("postgres://postgres:postgres@localhost/fragment".into()),
        pool_size: Some(10),
    })
    .await?;

let pool = databases.pool("primary")?;
let row = sqlx::query("SELECT COUNT(*) AS total FROM users")
    .fetch_one(&pool)
    .await?;
```

The SQLx manager exposes the same `connect` / `connect_all` / `pool` /
`close_all` surface as the ORM manager, so the two layers stay symmetrical.

## Migrations

SeaORM migrations live in their own crate. `frs make:migration` creates that
crate the first time it is used and adds a migration to it afterwards.

```sh
frs make:migration create_users_table
```

This creates a `migrations/` crate in your project, registers it as a
workspace member, adds it to your application dependencies, and writes the
migration file:

```text
migrations/
├── Cargo.toml
└── src/
    ├── lib.rs                        # the Migrator
    ├── main.rs                       # the migration CLI
    └── m0001_create_users_table.rs   # the migration
```

The generated migration follows the standard SeaORM shape:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tables::Table)
                    .col(ColumnDef::new(Tables::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Tables::Name).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tables::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tables {
    Table,
    Id,
    Name,
}
```

Each new migration gets the next sequence number and is added to the
`Migrator` automatically.

### Running migrations

The migration crate reads the `DATABASE_URL` environment variable:

```sh
DATABASE_URL="postgres://postgres:postgres@localhost/fragment" cargo run -p my-app-migrations -- up
```

| Command | Effect |
|---|---|
| `up` | Apply all pending migrations |
| `down` | Roll back the last applied migration |
| `status` | List applied and pending migrations |
| `fresh` | Drop everything and re-apply all migrations |

Point `DATABASE_URL` at one database at a time. The migration CLI operates on a
single connection, so run it once per named database.

## Multi-database examples

Reading from one database and writing to another:

```rust
let primary = databases.connection("primary")?;
let analytics = databases.connection("analytics")?;

let users = user::Entity::find().all(&primary).await?;

for user in users {
    event::ActiveModel {
        user_id: Set(user.id),
        ..Default::default()
    }
    .insert(&analytics)
    .await?;
}
```

Routing a tenant to a database:

```rust
fn database_for(tenant: &str) -> &'static str {
    match tenant {
        "acme" => "tenant_acme",
        "globex" => "tenant_globex",
        other => panic!("unknown tenant: {other}"),
    }
}

let connection = databases.connection(database_for(&tenant))?;
```

Never pass a client-supplied database name straight to `connection()`. Map the
tenant to a configured name first, as above, so a request cannot select an
arbitrary pool.

## See also

- [Configuration](CONFIGURATION.md) — `fragment.toml` and environment variables
- [CLI reference](CLI.md) — `frs make:entity`, `frs make:migration`
- [Framework usage guide](../USAGE.MD) — services, controllers, validation
