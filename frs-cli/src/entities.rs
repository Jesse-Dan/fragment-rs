//! Reads SeaORM entity definitions out of a project's source tree.
//!
//! The generator only needs the table name and the column list, so this
//! parser deliberately understands a small, well-defined subset of the entity
//! syntax: a struct carrying `#[sea_orm(table_name = "...")]` and one field
//! per column.

use std::path::{Path, PathBuf};

use syn::{Attribute, Fields, Item, ItemStruct, Type};

/// The column types the generator knows how to map onto SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Boolean,
    TinyInteger,
    SmallInteger,
    Integer,
    BigInteger,
    Float,
    Double,
    String,
    Timestamp,
    Uuid,
}

impl ColumnType {
    /// The SeaORM `ColumnDef` builder for this type.
    pub fn column_def(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::TinyInteger => "tiny_integer",
            Self::SmallInteger => "small_integer",
            Self::Integer => "integer",
            Self::BigInteger => "big_integer",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
            Self::Timestamp => "timestamp",
            Self::Uuid => "uuid",
        }
    }
}

/// A single column taken from an entity definition.
#[derive(Debug, Clone)]
pub struct EntityColumn {
    pub name: String,
    pub column_type: ColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
}

/// A table taken from an entity definition.
#[derive(Debug, Clone)]
pub struct EntityModel {
    pub table: String,
    pub columns: Vec<EntityColumn>,
}

/// Collects every entity definition found under `dir`.
///
/// Entity files are recognised by the `#[sea_orm(table_name = "...")]`
/// attribute, so unrelated modules in the same directory are ignored.
pub fn collect_entities(dir: &Path) -> Result<Vec<EntityModel>, String> {
    let mut entities = Vec::new();
    if !dir.is_dir() {
        return Ok(entities);
    }
    for file in rust_files(dir)? {
        let source = std::fs::read_to_string(&file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        let parsed = syn::parse_file(&source)
            .map_err(|error| format!("could not parse {}: {error}", file.display()))?;
        entities.extend(entities_in_file(&parsed));
    }
    entities.sort_by(|left, right| left.table.cmp(&right.table));
    Ok(entities)
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|error| format!("could not read {}: {error}", current.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("could not read a directory entry: {error}"))?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Extracts the entity models declared in a parsed Rust file.
fn entities_in_file(file: &syn::File) -> Vec<EntityModel> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => entity_model(item),
            _ => None,
        })
        .collect()
}

/// Reads one entity struct.
fn entity_model(item: &ItemStruct) -> Option<EntityModel> {
    let table = sea_orm_attr(&item.attrs, "table_name")?;
    let fields = match &item.fields {
        Fields::Named(fields) => fields,
        _ => return None,
    };

    let mut columns = Vec::new();
    for field in fields.named.iter() {
        let Some(name) = field.ident.as_ref() else {
            continue;
        };
        let Some((column_type, nullable)) = map_type(&field.ty) else {
            continue;
        };
        let primary_key = has_sea_orm_attr(&field.attrs, "primary_key");
        // SeaORM treats an integer primary key as auto-incrementing by
        // default, so the generated schema matches that behaviour.
        let auto_increment = has_sea_orm_attr(&field.attrs, "auto_increment")
            || (primary_key && column_type == ColumnType::Integer);
        columns.push(EntityColumn {
            name: name.to_string(),
            column_type,
            nullable,
            primary_key,
            unique: has_sea_orm_attr(&field.attrs, "unique"),
            auto_increment,
        });
    }

    if columns.is_empty() {
        return None;
    }
    Some(EntityModel { table, columns })
}

/// Reads the string value of `#[sea_orm(name = "...")]`.
fn sea_orm_attr(attrs: &[Attribute], name: &str) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("sea_orm") {
            continue;
        }
        let mut value = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(name) {
                let literal: syn::LitStr = meta.value()?.parse()?;
                value = Some(literal.value());
            }
            Ok(())
        });
        if value.is_some() {
            return value;
        }
    }
    None
}

/// Reports whether `#[sea_orm(name)]` is present.
fn has_sea_orm_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("sea_orm") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(name) {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// Maps a Rust field type onto a column type and its nullability.
fn map_type(ty: &Type) -> Option<(ColumnType, bool)> {
    if let Type::Path(path) = ty {
        // `Option<T>` is the only way a column becomes nullable.
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    if let Some(inner) = arguments.args.first() {
                        if let syn::GenericArgument::Type(inner) = inner {
                            return map_type(inner).map(|(kind, _)| (kind, true));
                        }
                    }
                }
                return None;
            }
        }
    }

    let last = match ty {
        Type::Path(path) => path.path.segments.last()?.ident.to_string(),
        Type::Reference(reference) => {
            let inner = match &*reference.elem {
                Type::Path(path) => path.path.segments.last()?.ident.to_string(),
                _ => return None,
            };
            inner
        }
        _ => return None,
    };

    let kind = match last.as_str() {
        "bool" => ColumnType::Boolean,
        "i8" => ColumnType::TinyInteger,
        "i16" => ColumnType::SmallInteger,
        "i32" => ColumnType::Integer,
        "i64" => ColumnType::BigInteger,
        "f32" => ColumnType::Float,
        "f64" => ColumnType::Double,
        "String" => ColumnType::String,
        "Uuid" => ColumnType::Uuid,
        "DateTime" | "DateTimeUtc" | "NaiveDateTime" => ColumnType::Timestamp,
        _ => return None,
    };
    Some((kind, false))
}

#[cfg(test)]
mod tests {
    use super::{collect_entities, ColumnType};
    use std::path::PathBuf;

    #[test]
    fn reads_table_name_and_columns_from_an_entity() {
        let dir = PathBuf::from(std::env::temp_dir())
            .join(format!("frs-entities-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("user.rs"),
            r#"
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub email: String,
    pub nickname: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#,
        )
        .unwrap();

        let entities = collect_entities(&dir).unwrap();
        assert_eq!(entities.len(), 1);
        let user = &entities[0];
        assert_eq!(user.table, "users");
        assert_eq!(user.columns.len(), 3);
        assert_eq!(user.columns[0].name, "id");
        assert_eq!(user.columns[0].column_type, ColumnType::Integer);
        assert!(user.columns[0].primary_key);
        assert_eq!(user.columns[1].column_type, ColumnType::String);
        assert!(!user.columns[1].nullable);
        assert!(user.columns[2].nullable);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ignores_files_without_entities() {
        let dir = PathBuf::from(std::env::temp_dir())
            .join(format!("frs-entities-empty-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.rs"), "//! Project module namespace.\n").unwrap();
        assert!(collect_entities(&dir).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
