# Fragment Rust

[![Release](https://img.shields.io/github/v/release/Jesse-Dan/fragment-rs)](https://github.com/Jesse-Dan/fragment-rs/releases/latest)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

Fragment Rust is a Spring Boot-inspired web framework built on Axum. It provides a Rust CLI, application scaffolding, controller and service macros, validation, structured errors, and SeaORM-backed multi-database access.

> **Status:** Fragment Rust is under active development. Review the limitations in [the project structure guide](docs/PROJECT-STRUCTURE.md) before depending on features in production.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

Windows:

```powershell
irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
```

This installs the latest release of the `frs` CLI into `~/.local/bin` and puts it on your `PATH`. See [Installation](docs/INSTALL.md) for the other options.

## Start here

1. [Install the `frs` CLI](docs/INSTALL.md).
2. [Create an application](docs/CLI.md#frs-create).
3. [Choose a project layout](docs/PROJECT-STRUCTURE.md).
4. [Generate modules and files](docs/CLI.md#code-generation).
5. [Use SeaORM entities and multiple databases](docs/ORM.md).

## Quick start

```sh
frs --version
frs create storefront --structure module
cd storefront
cargo run
```

The CLI creates a runnable Rust application with a `fragment.toml` structure setting, source folders, and a starter app module. Use `frs help` to see CLI commands.

## A controller and a service

```rust
use axum::Json;
use fragment_rs::{controller, route, service, FragmentApp, Inject, GET};

#[service]
#[derive(Default)]
struct CustomerRepository {
    customers: std::sync::Arc<std::sync::Mutex<Vec<Customer>>>,
}

struct CustomerController;

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepository>) -> Json<Vec<Customer>> {
        Json(repo.customers.lock().unwrap().clone())
    }
}

#[tokio::main]
async fn main() {
    FragmentApp::new().port(3000).start().await;
}
```

Controllers and services register themselves, so there is nothing to wire up.
`GET /api/customers` answers with the list. The prefix and the route path are
joined without a trailing slash, so `#[route(GET, "/")]` under
`/api/customers` serves `/api/customers`.

## Generate a module

```sh
# In a project created with --structure module or --structure feature:
frs make:mod authentication

# Add a nested module below src/modules/api/:
frs make:mod authentication --pmod=api

# Add components to the selected module:
frs make:controller login --pmod=authentication
frs make:service authentication --pmod=api
frs make:entity user
frs check
```

`--pmod` is an optional parent path. Nested parent paths may use `/`, for example `--pmod=api/auth`.

## Migrations

`frs make:migration` creates a SeaORM migration crate in your project and
writes a migration into it:

```sh
frs make:migration create_users_table
DATABASE_URL="postgres://localhost/fragment" cargo run -p my-app-migrations -- up
```

`frs make:migration -g` goes further: it compares your entity definitions with
the live database schema and writes the migration for the difference.

```sh
frs make:migration -g --database-url postgres://localhost/fragment
```

The database is only read. The generated migration creates missing tables and
adds missing columns; it never drops anything, and objects it does not
recognise are reported and left alone.

## Layouts

Create with `--structure layered`, `--structure module`, or `--structure feature`. The default is `module`.

- **Layered:** folders by technical role (`controllers`, `services`, `repositories`, and more).
- **Module:** domain modules under `src/modules/`, each with its controllers and services.
- **Feature:** business areas under `src/features/`, alongside shared infrastructure.

See [Project structure](docs/PROJECT-STRUCTURE.md) for full trees, generator behavior, and how to choose a layout.

## Database and ORM

SeaORM provides entity derives, ActiveModels, relationships, queries, and transactions. `OrmDatabaseManager` manages named connections; SQLx pools are available for raw SQL.

```rust
let db = OrmDatabaseManager::new();
db.connect_all(&config.databases).await?;
let primary = db.connection("primary")?;
let users = user::Entity::find().all(&primary).await?;
```

See [ORM and databases](docs/ORM.md), and run `cargo run -p fragment-rs --example orm_entities` in this repository for an entity CRUD walkthrough.

## Documentation

- [Installation](docs/INSTALL.md) — install `frs` on macOS, Linux, or Windows.
- [CLI reference](docs/CLI.md) — commands and examples.
- [Project structure](docs/PROJECT-STRUCTURE.md) — layout modes and generated files.
- [ORM and databases](docs/ORM.md) — entities, queries, named connections, and migrations.
- [Configuration](docs/CONFIGURATION.md) — `fragment.toml`, environment variables, and server settings.
- [Framework usage guide](USAGE.MD) — services, controllers, validation, errors, and deployment.
- [Development and releases](docs/DEVELOPMENT.md) — tests, local CLI development, and tagged GitHub releases.

## Development status and contributions

The CLI scaffolds structure and Rust source files; generated controller/service files are starting points and require application logic. It does not currently perform automatic component discovery, generate runnable HTTP route handlers, or execute migrations. Contributions and issue reports are welcome; see [Development and releases](docs/DEVELOPMENT.md).
