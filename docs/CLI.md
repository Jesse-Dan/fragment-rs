# `frs` CLI reference

Run `frs help` for a short command list. Commands that generate files must be run from a Fragment project directory containing `Cargo.toml` and `fragment.toml`, unless the command is `frs create`.

## `frs create`

Create a new app and select its layout:

```sh
frs create my-api
frs create storefront --structure module
frs create billing --structure feature
frs create legacy-service --structure layered
```

Available options:

```text
frs create <name> [--structure layered|module|feature] [--path DIRECTORY]
```

- `--structure` / `-s`: project layout; defaults to `module`.
- `--path` / `-p`: parent directory in which to create the project; defaults to the current directory.
- `new` is an alias for `create`.

Examples:

```sh
frs create customer-api --path ./services --structure module
cd services/customer-api
cargo run
```

The CLI refuses to overwrite an existing destination. The generated app contains `Cargo.toml`, `fragment.toml`, `README.md`, and a runnable `src/main.rs`.

## Code generation

### Create a domain module

For module and feature layouts:

```sh
frs make:mod authentication
frs make:mod billing
frs make:mod tokens --pmod=authentication
frs make:mod oauth --pmod=authentication/providers
```

Both `--pmod PARENT` and `--pmod=PARENT` are accepted. Use `/` to express nested parent folders. Missing parent modules are scaffolded. Names are normalized to Rust `snake_case` for modules and `PascalCase` for generated types. Existing module directories are not overwritten.

A module contains controllers and services, for example:

```text
src/modules/authentication/
├── mod.rs
├── controllers/
│   ├── mod.rs
│   └── authentication_controller.rs
└── services/
    ├── mod.rs
    └── authentication_service.rs
```

For feature layout, the same structure is generated under `src/features/`.

`frs make:mod` is intentionally not available in layered projects. In a layered project, generate each responsibility directly into its layer with `make:controller` and `make:service`.

### Generate controller, service, or entity files

```sh
# Module/feature layout: no --pmod means the starter app module.
frs make:controller login
frs make:service authentication --pmod=authentication

# Layered layout: files go into their technical layer.
frs make:controller health
frs make:service users

# SeaORM entity; entities are stored in src/entities/.
frs make:entity user
```

Generated controllers and services are Rust type stubs, not automatically registered or functional HTTP endpoints. Add application behavior and wire routes/services explicitly. See [framework usage](../USAGE.MD) and [ORM usage](ORM.md).

### Generate a migration scaffold

```sh
frs make:migration create_users_table
```

This creates a `migrations/` crate in your project if it does not exist yet,
registers it as a workspace member, adds it to your application dependencies,
and writes the migration file.

```text
migrations/
├── Cargo.toml
└── src/
    ├── lib.rs                        # the Migrator
    ├── main.rs                       # the migration CLI
    └── m0001_create_users_table.rs   # the migration
```

Each migration is numbered in order, and the crate's `Migrator` is updated to
include it. Run the migrations with:

```sh
DATABASE_URL="postgres://localhost/fragment" cargo run -p my-app-migrations -- up
```

### Generate a migration from your entities

```sh
frs make:migration -g
```

The `-g` flag compares the entity definitions in `src/entities` with the live
database schema and writes a migration for the difference.

```text
frs make:migration -g --database-url postgres://localhost/fragment
frs make:migration -g --database primary
frs make:migration -g --entities src/domain/entities
```

The database is resolved in this order:

1. `--database-url <url>`
2. the `DATABASE_URL` environment variable
3. the database called `--database <name>` in `fragment.toml`

> **The database is only read.** Generating a migration never changes the
> database and never applies a migration. The generated `up` creates missing
> tables and adds missing columns; it never drops anything. Tables and columns
> that exist in the database but are not described by an entity are reported
> and left alone.

If the database already matches the entities, no file is written.

## Validate and inspect a project

```sh
frs check
frs check --path ./services/customer-api
frs doctor
frs --version
frs help
```

- `check` executes `cargo check` in the selected project directory.
- `doctor` reports the installed `rustc` and `cargo` versions.
- `--path` / `-p` selects a project directory for `check`.

## Full workflow example

```sh
frs create identity-api --structure module
cd identity-api
frs make:mod authentication
frs make:mod providers --pmod=authentication
frs make:controller login --pmod=authentication
frs make:service tokens --pmod=authentication
frs make:entity user
frs make:migration create_users_table
frs check
cargo run
```

Next steps after scaffolding:

1. Implement controller/service behavior in generated `.rs` files.
2. Add route registration and application state in `src/main.rs`.
3. Define entities and relationships, then configure named database connections.
4. Generate migrations with `frs make:migration -g` and run them with
   `cargo run -p identity-api-migrations -- up`.
5. Run `cargo fmt`, `cargo clippy --workspace --all-targets`, and `cargo test` before committing.
