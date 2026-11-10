# fragment-rs

A Spring Boot-inspired web framework for Rust powered by Axum. Zero config, zero manual registration. Auto dependency injection, validation, and OpenAPI documentation out of the box.

[![Crates.io](https://img.shields.io/crates/v/fragment-rs.svg)](https://crates.io/crates/fragment-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

## Quick Start

```bash
cargo new my-api && cd my-api
# Add to Cargo.toml:
[dependencies]
fragment-rs = "0.2"
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
```

```rust
use fragment_rs::{service, controller, route};
use axum::Json;

#[service]
struct CustomerRepo { /* ... */ }

impl Default for CustomerRepo { fn default() -> Self { /* init DB */ } }

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepo>) -> Json<Vec<Customer>> { /* ... */ }
}

#[tokio::main]
async fn main() {
    FragmentApp::new().port(3000).start().await;
}
```

Run: `cargo run` → http://localhost:3000/api/customers/

## Features

- **Auto Dependency Injection** — `#[service]` registers singletons, `Inject<T>` extracts them anywhere.
- **Zero Config Routing** — `#[controller("/api/v1")]` + `#[route(GET, "/")]` compiles to Axum routes.
- **Built-in Validation** — `ValidatedJson<T>` with the `validator` crate and `#[derive(Validate)]`.
- **Global Error Handling** — `#[exception_filter]` catches panics/errors into structured JSON responses.
- **OpenAPI Docs** — Automatically generated from route metadata.

## Documentation

- [Usage Guide](USAGE.MD)
- [Architecture Deep Dive](ARCHITECTURE.md)
