//! Reads a live database schema and turns the difference between it and the
//! project's entity definitions into a SeaORM migration.
//!
//! # Safety
//!
//! Everything in this module is read-only with respect to the database: the
//! generator inspects the schema and writes files, it never issues a DDL
//! statement against the database and never applies a migration.
//!
//! The generated `up` only performs additive work — creating missing tables
//! and adding missing columns. Objects that exist in the database but are not
//! described by an entity are reported and left untouched, because dropping
//! them would destroy data the developer may still need.

use sea_orm::sea_query::Value;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, QueryResult, Statement};

use crate::entities::{ColumnType, EntityColumn, EntityModel};

/// A column as it exists in the live database.
#[derive(Debug, Clone)]
pub struct DatabaseColumn {
    pub name: String,
    pub nullable: bool,
    pub column_type: String,
}

/// A table as it exists in the live database.
#[derive(Debug, Clone)]
pub struct DatabaseTable {
    pub name: String,
    pub columns: Vec<DatabaseColumn>,
}

/// A single additive change between the entities and the database.
#[derive(Debug, Clone)]
pub enum Change {
    /// The table does not exist yet.
    CreateTable(EntityModel),
    /// The table exists but the column does not.
    AddColumn { table: String, column: EntityColumn },
}

/// The result of comparing entity definitions with the live schema.
#[derive(Debug, Default)]
pub struct Diff {
    pub changes: Vec<Change>,
    /// Differences that are reported but never turned into DDL.
    pub notes: Vec<String>,
}

/// Opens a connection and reads the schema of every table it contains.
pub async fn introspect(url: &str) -> Result<Vec<DatabaseTable>, String> {
    let connection = Database::connect(url)
        .await
        .map_err(|error| format!("could not connect to the database: {error}"))?;
    let tables = read_schema(&connection).await?;
    connection
        .close()
        .await
        .map_err(|error| format!("could not close the database connection: {error}"))?;
    Ok(tables)
}

/// Reads the schema of every table in the database.
async fn read_schema(connection: &DatabaseConnection) -> Result<Vec<DatabaseTable>, String> {
    match connection.get_database_backend() {
        DbBackend::Sqlite => read_sqlite_schema(connection).await,
        DbBackend::Postgres => read_postgres_schema(connection).await,
        DbBackend::MySql => read_mysql_schema(connection).await,
    }
}

/// Runs a read-only query and returns the rows.
async fn query_all(
    connection: &DatabaseConnection,
    sql: &str,
    values: Vec<String>,
) -> Result<Vec<QueryResult>, String> {
    let backend = connection.get_database_backend();
    let values = values
        .into_iter()
        .map(|value| Value::String(Some(Box::new(value))))
        .collect::<Vec<_>>();
    let statement = Statement::from_sql_and_values(backend, sql, values);
    connection
        .query_all(statement)
        .await
        .map_err(|error| format!("query failed ({sql}): {error}"))
}

/// Reads a string column, tolerating drivers that report it differently.
fn text(row: &QueryResult, column: &str) -> String {
    row.try_get::<String>("", column)
        .or_else(|_| row.try_get_by_index::<String>(0))
        .unwrap_or_default()
}

/// Reports whether a table belongs to SeaORM rather than to the application.
///
/// SeaORM keeps its own migration history in `seaql_migrations`; it is not
/// part of the application's schema and must never be reported as unmanaged.
fn is_internal_table(name: &str) -> bool {
    name == "seaql_migrations"
}

/// Reads the schema of a SQLite database.
async fn read_sqlite_schema(connection: &DatabaseConnection) -> Result<Vec<DatabaseTable>, String> {
    let rows = query_all(
        connection,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        Vec::new(),
    )
    .await?;

    let mut tables = Vec::new();
    for row in rows {
        let name = text(&row, "name");
        if name.is_empty() || is_internal_table(&name) {
            continue;
        }
        // `PRAGMA table_info` does not accept bound parameters, so the table
        // name is quoted here. The name comes from the project's own entity
        // definitions, and any quote inside it is doubled.
        let quoted = name.replace('"', "\"\"");
        let columns = query_all(
            connection,
            &format!("PRAGMA table_info(\"{quoted}\")"),
            Vec::new(),
        )
        .await?;

        let mut parsed = Vec::new();
        for column in columns {
            let column_name = text(&column, "name");
            if column_name.is_empty() {
                continue;
            }
            let not_null = column
                .try_get::<bool>("", "notnull")
                .or_else(|_| column.try_get::<i64>("", "notnull").map(|value| value != 0))
                .unwrap_or(false);
            parsed.push(DatabaseColumn {
                name: column_name,
                nullable: !not_null,
                column_type: text(&column, "type"),
            });
        }
        tables.push(DatabaseTable {
            name,
            columns: parsed,
        });
    }
    Ok(tables)
}

/// Reads the schema of a PostgreSQL database.
async fn read_postgres_schema(
    connection: &DatabaseConnection,
) -> Result<Vec<DatabaseTable>, String> {
    let rows = query_all(
        connection,
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name",
        Vec::new(),
    )
    .await?;

    let mut tables = Vec::new();
    for row in rows {
        let name = text(&row, "table_name");
        if name.is_empty() || is_internal_table(&name) {
            continue;
        }
        let columns = query_all(
            connection,
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position",
            vec![name.clone()],
        )
        .await?;

        let mut parsed = Vec::new();
        for column in columns {
            let column_name = text(&column, "column_name");
            if column_name.is_empty() {
                continue;
            }
            let nullable = text(&column, "is_nullable").eq_ignore_ascii_case("YES");
            parsed.push(DatabaseColumn {
                name: column_name,
                nullable,
                column_type: text(&column, "data_type"),
            });
        }
        tables.push(DatabaseTable {
            name,
            columns: parsed,
        });
    }
    Ok(tables)
}

/// Reads the schema of a MySQL database.
async fn read_mysql_schema(connection: &DatabaseConnection) -> Result<Vec<DatabaseTable>, String> {
    let rows = query_all(
        connection,
        "SELECT TABLE_NAME FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME",
        Vec::new(),
    )
    .await?;

    let mut tables = Vec::new();
    for row in rows {
        let name = text(&row, "TABLE_NAME");
        if name.is_empty() || is_internal_table(&name) {
            continue;
        }
        let columns = query_all(
            connection,
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
            vec![name.clone()],
        )
        .await?;

        let mut parsed = Vec::new();
        for column in columns {
            let column_name = text(&column, "COLUMN_NAME");
            if column_name.is_empty() {
                continue;
            }
            let nullable = text(&column, "IS_NULLABLE").eq_ignore_ascii_case("YES");
            parsed.push(DatabaseColumn {
                name: column_name,
                nullable,
                column_type: text(&column, "DATA_TYPE"),
            });
        }
        tables.push(DatabaseTable {
            name,
            columns: parsed,
        });
    }
    Ok(tables)
}

/// Compares the entity definitions with the live schema.
///
/// Only additive differences become changes. Everything else is reported as a
/// note so the developer can decide what to do.
pub fn diff(entities: &[EntityModel], database: &[DatabaseTable]) -> Diff {
    let mut changes = Vec::new();
    let mut notes = Vec::new();

    for entity in entities {
        let existing = database.iter().find(|table| table.name == entity.table);
        let Some(existing) = existing else {
            changes.push(Change::CreateTable(entity.clone()));
            continue;
        };

        for column in &entity.columns {
            match existing
                .columns
                .iter()
                .find(|item| item.name == column.name)
            {
                None => changes.push(Change::AddColumn {
                    table: entity.table.clone(),
                    column: column.clone(),
                }),
                Some(item) => {
                    if !types_agree(column.column_type, &item.column_type) {
                        notes.push(format!(
                            "column `{}.{}` is `{}` in the entity but `{}` in the database; change it by hand",
                            entity.table, column.name, column.column_type.column_def(), item.column_type
                        ));
                    }
                    if column.nullable != item.nullable {
                        notes.push(format!(
                            "column `{}.{}` is {} in the entity but {} in the database; change it by hand",
                            entity.table,
                            column.name,
                            if column.nullable { "nullable" } else { "not null" },
                            if item.nullable { "nullable" } else { "not null" }
                        ));
                    }
                }
            }
        }

        for item in &existing.columns {
            if !entity.columns.iter().any(|column| column.name == item.name) {
                notes.push(format!(
                    "column `{}.{}` exists in the database but not in the entity; it is left alone",
                    entity.table, item.name
                ));
            }
        }
    }

    for table in database {
        if !entities.iter().any(|entity| entity.table == table.name) {
            notes.push(format!(
                "table `{}` exists in the database but is not described by an entity; it is left alone",
                table.name
            ));
        }
    }

    Diff { changes, notes }
}

/// Compares an entity column type with the type reported by the database.
fn types_agree(entity: ColumnType, database: &str) -> bool {
    normalize_database_type(database) == normalize_entity_type(entity)
}

/// The comparable name of a type reported by a database.
fn normalize_database_type(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "int" | "int4" | "integer" | "int32" | "mediumint" => "integer",
        "smallint" | "int2" | "smallserial" => "smallint",
        "bigint" | "int8" | "bigserial" => "bigint",
        "tinyint" => "tinyint",
        "bool" | "boolean" => "boolean",
        "text" | "varchar" | "character varying" | "string" | "nvarchar" | "nchar" => "string",
        "timestamptz"
        | "timestamp with time zone"
        | "timestamp without time zone"
        | "timestamp"
        | "datetime"
        | "datetime2" => "timestamp",
        "uuid" => "uuid",
        "real" | "float" | "float4" => "float",
        "double precision" | "double" | "float8" => "double",
        other => other,
    }
    .to_owned()
}

/// The comparable name of a type declared by an entity.
fn normalize_entity_type(entity: ColumnType) -> String {
    match entity {
        ColumnType::Boolean => "boolean",
        ColumnType::TinyInteger => "tinyint",
        ColumnType::SmallInteger => "smallint",
        ColumnType::Integer => "integer",
        ColumnType::BigInteger => "bigint",
        ColumnType::Float => "float",
        ColumnType::Double => "double",
        ColumnType::String => "string",
        ColumnType::Timestamp => "timestamp",
        ColumnType::Uuid => "uuid",
    }
    .to_owned()
}

/// Renders the migration file for a set of changes.
///
/// The `up` only creates missing tables and adds missing columns. The `down`
/// reverses exactly those operations and nothing else.
pub fn render_migration(changes: &[Change], name: &str) -> String {
    let mut tables: Vec<(String, Vec<String>)> = Vec::new();
    for change in changes {
        match change {
            Change::CreateTable(entity) => {
                let columns = entity
                    .columns
                    .iter()
                    .map(|column| column.name.clone())
                    .collect();
                upsert(&mut tables, &entity.table, columns);
            }
            Change::AddColumn { table, column } => {
                let columns = vec![column.name.clone()];
                upsert(&mut tables, table, columns);
            }
        }
    }

    let mut idents = String::new();
    let mut table_idents: Vec<(String, String)> = Vec::new();
    for (table, columns) in &tables {
        let enum_name = pascal_case(table);
        let mut variants = vec![format!("    Table,")];
        for column in columns {
            let variant = pascal_case(column);
            if variants
                .iter()
                .any(|existing| existing.trim_end_matches(',') == variant)
            {
                continue;
            }
            variants.push(format!("    {variant},"));
        }
        idents.push_str(&format!(
            "#[derive(DeriveIden)]\nenum {enum_name} {{\n{}\n}}\n\n",
            variants.join("\n")
        ));
        table_idents.push((table.clone(), enum_name));
    }

    let up = render_up(changes, &table_idents);
    let down = render_down(changes, &table_idents);

    format!(
        "use sea_orm_migration::prelude::*;\n\n{idents}#[derive(DeriveMigrationName)]\npub struct Migration;\n\n#[async_trait::async_trait]\nimpl MigrationTrait for Migration {{\n    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{\n{up}        Ok(())\n    }}\n\n    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{\n{down}        Ok(())\n    }}\n}}\n\n// Generated by `frs make:migration -g` from the entity definitions and the \
live database schema.\n// Migration: {name}\n"
    )
}

/// Adds a table and its columns to the collected list.
fn upsert(tables: &mut Vec<(String, Vec<String>)>, table: &str, columns: Vec<String>) {
    match tables.iter_mut().find(|(name, _)| name == table) {
        Some((_, existing)) => {
            for column in columns {
                if !existing.contains(&column) {
                    existing.push(column);
                }
            }
        }
        None => tables.push((table.to_owned(), columns)),
    }
}

/// Renders the `up` body.
fn render_up(changes: &[Change], tables: &[(String, String)]) -> String {
    let mut body = String::new();
    for change in changes {
        match change {
            Change::CreateTable(entity) => {
                let enum_name = ident_for(&entity.table, tables);
                body.push_str("        manager\n            .create_table(\n                Table::create()\n");
                body.push_str(&format!("                    .table({enum_name}::Table)\n"));
                for column in &entity.columns {
                    body.push_str(&format!(
                        "                    .col({})\n",
                        column_def(&enum_name, column)
                    ));
                }
                body.push_str(
                    "                    .to_owned(),\n            )\n            .await?;\n",
                );
            }
            Change::AddColumn { table, column } => {
                let enum_name = ident_for(table, tables);
                body.push_str(
                    "        manager\n            .alter_table(\n                Table::alter()\n",
                );
                body.push_str(&format!("                    .table({enum_name}::Table)\n"));
                body.push_str(&format!(
                    "                    .add_column({})\n                    .to_owned(),\n            )\n            .await?;\n",
                    column_def(&enum_name, column)
                ));
            }
        }
    }
    body
}

/// Renders the `down` body, reversing exactly what `up` does.
fn render_down(changes: &[Change], tables: &[(String, String)]) -> String {
    let mut body = String::new();
    for change in changes.iter().rev() {
        match change {
            Change::CreateTable(entity) => {
                let enum_name = ident_for(&entity.table, tables);
                body.push_str(&format!(
                    "        manager\n            .drop_table(Table::drop().table({enum_name}::Table).to_owned())\n            .await?;\n"
                ));
            }
            Change::AddColumn { table, column } => {
                let enum_name = ident_for(table, tables);
                body.push_str(&format!(
                    "        manager\n            .alter_table(\n                Table::alter()\n                    .table({enum_name}::Table)\n                    .drop_column({enum_name}::{})\n                    .to_owned(),\n            )\n            .await?;\n",
                    pascal_case(&column.name)
                ));
            }
        }
    }
    body
}

/// The `DeriveIden` enum name used for a table.
fn ident_for(table: &str, tables: &[(String, String)]) -> String {
    tables
        .iter()
        .find(|(name, _)| name == table)
        .map(|(_, enum_name)| enum_name.clone())
        .unwrap_or_else(|| pascal_case(table))
}

/// Renders the `ColumnDef` for one column.
fn column_def(enum_name: &str, column: &EntityColumn) -> String {
    let variant = pascal_case(&column.name);
    let mut definition = format!(
        "ColumnDef::new({enum_name}::{variant}).{}()",
        column.column_type.column_def()
    );
    if column.primary_key && column.auto_increment {
        definition.push_str(".not_null().auto_increment().primary_key()");
    } else if column.primary_key {
        definition.push_str(".not_null().primary_key()");
    } else if column.nullable {
        definition.push_str(".null()");
    } else {
        definition.push_str(".not_null()");
    }
    if column.unique {
        definition.push_str(".unique_key()");
    }
    definition
}

/// Converts a snake_case name into PascalCase.
fn pascal_case(value: &str) -> String {
    let mut result = String::new();
    for part in value.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{diff, render_migration, types_agree};
    use crate::entities::{ColumnType, EntityColumn, EntityModel};

    fn entity() -> EntityModel {
        EntityModel {
            table: "users".to_owned(),
            columns: vec![
                EntityColumn {
                    name: "id".to_owned(),
                    column_type: ColumnType::Integer,
                    nullable: false,
                    primary_key: true,
                    unique: false,
                    auto_increment: true,
                },
                EntityColumn {
                    name: "email".to_owned(),
                    column_type: ColumnType::String,
                    nullable: false,
                    primary_key: false,
                    unique: false,
                    auto_increment: false,
                },
            ],
        }
    }

    #[test]
    fn missing_table_becomes_a_create() {
        let result = diff(&[entity()], &[]);
        assert_eq!(result.changes.len(), 1);
        assert!(result.notes.is_empty());
    }

    #[test]
    fn missing_column_becomes_an_add() {
        let database = vec![super::DatabaseTable {
            name: "users".to_owned(),
            columns: vec![super::DatabaseColumn {
                name: "id".to_owned(),
                nullable: false,
                column_type: "integer".to_owned(),
            }],
        }];
        let result = diff(&[entity()], &database);
        assert_eq!(result.changes.len(), 1);
        assert!(matches!(result.changes[0], super::Change::AddColumn { .. }));
    }

    #[test]
    fn unmanaged_columns_are_reported_but_never_dropped() {
        let database = vec![super::DatabaseTable {
            name: "users".to_owned(),
            columns: vec![
                super::DatabaseColumn {
                    name: "id".to_owned(),
                    nullable: false,
                    column_type: "integer".to_owned(),
                },
                super::DatabaseColumn {
                    name: "email".to_owned(),
                    nullable: false,
                    column_type: "character varying".to_owned(),
                },
                super::DatabaseColumn {
                    name: "legacy".to_owned(),
                    nullable: true,
                    column_type: "text".to_owned(),
                },
            ],
        }];
        let result = diff(&[entity()], &database);
        assert!(result.changes.is_empty());
        assert_eq!(result.notes.len(), 1);
        assert!(result.notes[0].contains("legacy"));
    }

    #[test]
    fn unmanaged_tables_are_reported_but_never_dropped() {
        let database = vec![super::DatabaseTable {
            name: "audit_log".to_owned(),
            columns: Vec::new(),
        }];
        let result = diff(&[entity()], &database);
        assert_eq!(result.changes.len(), 1);
        assert!(result.notes.iter().any(|note| note.contains("audit_log")));
    }

    #[test]
    fn type_drift_is_reported_but_not_generated() {
        let database = vec![super::DatabaseTable {
            name: "users".to_owned(),
            columns: vec![
                super::DatabaseColumn {
                    name: "id".to_owned(),
                    nullable: false,
                    column_type: "integer".to_owned(),
                },
                super::DatabaseColumn {
                    name: "email".to_owned(),
                    nullable: false,
                    column_type: "integer".to_owned(),
                },
            ],
        }];
        let result = diff(&[entity()], &database);
        assert!(result.changes.is_empty());
        assert!(result.notes.iter().any(|note| note.contains("email")));
    }

    #[test]
    fn database_type_names_are_normalised() {
        assert!(types_agree(ColumnType::Integer, "int4"));
        assert!(types_agree(ColumnType::String, "character varying"));
        assert!(types_agree(
            ColumnType::Timestamp,
            "timestamp without time zone"
        ));
        assert!(types_agree(ColumnType::Uuid, "uuid"));
        assert!(!types_agree(ColumnType::Integer, "bigint"));
    }

    #[test]
    fn generated_migration_only_adds() {
        let migration = render_migration(&diff(&[entity()], &[]).changes, "create_users");
        assert!(migration.contains(".create_table("));
        assert!(migration.contains("auto_increment().primary_key()"));
        assert!(migration.contains(".drop_table("));
        assert!(!migration.contains("drop_column"));
    }
}
