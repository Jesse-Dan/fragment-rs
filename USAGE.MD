# fragment-rs: Developer Guide

A Spring Boot-inspired framework for Rust powered by Axum. Zero config, zero manual registration.

## Table of Contents

- [Quick Start](#quick-start)
- [Project Setup](#project-setup)
- [Writing Services](#writing-services)
- [Writing Controllers](#writing-controllers)
- [Dependency Injection](#dependency-injection)
- [Validation](#validation)
- [Error Handling](#error-handling)
- [Running the App](#running-the-app)
- [Writing Tests](#writing-tests)
- [Deployment](#deployment)

---

## Quick Start

```toml
[dependencies]
fragment-rs = { path = "../fragment-rs" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
validator = { version = "0.18", features = ["derive"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

---

## Project Setup

```bash
cargo new my-api --bin
cd my-api
```

Add `fragment-rs` as a path dependency:

```toml
[dependencies]
fragment-rs = { path = "../fragment-rs" }
```

---

## Writing Services

`#[service]` auto-registers the type as a singleton.

```rust
use fragment_rs::service;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct Customer {
    id: String,
    name: String,
    email: String,
}

#[service]
struct CustomerRepository {
    customers: Arc<Mutex<Vec<Customer>>>,
}

impl CustomerRepository {
    fn new() -> Self {
        Self {
            customers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn list(&self) -> Vec<Customer> {
        self.customers.lock().unwrap().clone()
    }

    pub fn create(&self, customer: Customer) {
        self.customers.lock().unwrap().push(customer);
    }
}

impl Default for CustomerRepository {
    fn default() -> Self { Self::new() }
}
```

### Injectable with Custom Scope

```rust
use fragment_rs::injectable;

#[injectable]
struct RequestScopedService {
    data: String,
}

impl RequestScopedService {
    fn new() -> Self {
        Self { data: "request-scoped".into() }
    }
}
```

---

## Writing Controllers

`#[controller]` auto-registers routes. `#[route]` marks each handler.

```rust
use fragment_rs::{controller, route};
use fragment_rs::core::container::Inject;
use fragment_rs::core::validation::ValidatedJson;
use axum::{http::StatusCode, Json};

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list_customers(
        Inject(repo): Inject<CustomerRepository>,
    ) -> Json<Vec<Customer>> {
        Json(repo.list())
    }

    #[route(POST, "/")]
    async fn create_customer(
        Inject(repo): Inject<CustomerRepository>,
        ValidatedJson(body): ValidatedJson<CreateCustomerDto>,
    ) -> (StatusCode, Json<Customer>) {
        let customer = Customer {
            id: uuid::Uuid::new_v4().to_string(),
            name: body.name,
            email: body.email.clone(),
        };
        repo.create(customer.clone());
        (StatusCode::CREATED, Json(customer))
    }
}
```

### Supported HTTP Methods

`GET`, `POST`, `PUT`, `DELETE`, `PATCH`

---

## Dependency Injection

Use `Inject<T>` as an Axum extractor.

```rust
async fn my_handler(
    Inject(service): Inject<MyService>,
) -> impl IntoResponse {
    // ...
}
```

### Built-in Extractors

| Type | Description |
|------|-------------|
| `Inject<T>` | Resolves `T` from DI container |
| `ValidatedJson<T>` | JSON body with validator checks |
| `ValidatedQuery<T>` | Query params with validator checks |
| `axum::extract::Path<T>` | URL path parameters |
| `axum::extract::Query<T>` | URL query string |
| `axum::extract::State<S>` | Shared application state |

---

## Validation

Use `#[derive(Validate)]` on DTOs, then `ValidatedJson<T>`:

```rust
use validator::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
struct CreateCustomerDto {
    #[validate(length(min = 2, message = "Name must be at least 2 characters"))]
    name: String,
    #[validate(email(message = "Invalid email address"))]
    email: String,
}
```

---

## Error Handling

`#[exception_filter]` auto-registers a global error handler:

```rust
use fragment_rs::exception_filter;

#[exception_filter]
struct GlobalExceptionFilter;

impl Default for GlobalExceptionFilter {
    fn default() -> Self { Self }
}
```

---

## Running the App

`FragmentApp` handles everything — logging, service registration, controller wiring, and server startup.

```rust
use fragment_rs::FragmentApp;

#[tokio::main]
async fn main() {
    FragmentApp::new()
        .host("0.0.0.0")
        .port(3000)
        .cors_origins(vec!["http://localhost:3001".into()])
        .file_upload_max_size(10 * 1024 * 1024)
        .start()
        .await;
}
```

### FragmentApp Configuration

```rust
FragmentApp::new()
    .host("0.0.0.0")                      // bind address (default: "0.0.0.0")
    .port(3000)                           // port (default: 3000)
    .auto_scan(true)                      // auto-register components (default: true)
    .compression(true)                    // gzip compression (default: false)
    .cors_origins(vec!["*".into()])      // CORS origins
    .cors_credentials(true)              // allow credentials (default: false)
    .helmet(true)                         // security headers (default: false)
    .file_upload_max_size(10 * 1024 * 1024) // max file size in bytes
    .file_upload_allowed_types(vec!["image/png".into(), "image/jpeg".into()])
    .file_upload_max_files(10)            // max files per request
    .start()
    .await;
```

Run:

```bash
cargo run --example customers_api
# or
cargo build --release && ./target/release/my-api
```

---

## Writing Tests

Services and controllers auto-register. Tests build the app and fire requests directly.

### Unit Testing

```rust
use fragment_rs::core::container::DIContainer;

#[service]
struct TestService {
    value: i32,
}

impl Default for TestService {
    fn default() -> Self { Self { value: 42 } }
}

#[test]
fn test_service_resolved() {
    let service = DIContainer::get::<TestService>();
    assert!(service.is_some());
    assert_eq!(service.unwrap().value, 42);
}
```

### Integration Testing

```rust
use fragment_rs::{controller, route, FragmentApp, core::container::Inject};
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[service]
struct TestCustomerRepo {
    customers: std::sync::Arc<std::sync::Mutex<Vec<Customer>>>,
}

impl TestCustomerRepo {
    fn new() -> Self {
        Self {
            customers: std::sync::Arc::new(std::sync::Mutex::new(vec![
                Customer { id: "1".into(), name: "Alice".into() },
            ])),
        }
    }
}

impl Default for TestCustomerRepo {
    fn default() -> Self { Self::new() }
}

#[controller("/api/customers")]
impl TestCustomerController {
    #[route(GET, "/")]
    async fn list(
        Inject(repo): Inject<TestCustomerRepo>,
    ) -> Json<Vec<Customer>> {
        let customers = repo.customers.lock().unwrap().clone();
        Json(customers)
    }
}

#[tokio::test]
async fn test_list_customers() {
    use http::Request;
    use tower::ServiceExt;

    let mut router = axum::Router::new();
    for ctrl in fragment_rs::core::container::DIContainer::auto_register_controllers() {
        router = ctrl.register_routes(router);
    }

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/customers/")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let customers: Vec<Customer> = serde_json::from_slice(&body).unwrap();
    assert_eq!(customers.len(), 1);
}
```

Run tests:

```bash
cargo test
```

---

## Deployment

### Build for Release

```bash
cargo build --release
```

Binary lands at `./target/release/my-api`.

### Docker

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my-api /app/my-api
EXPOSE 3000
CMD ["/app/my-api"]
```

```bash
docker build -t my-api .
docker run -p 3000:3000 my-api
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Tracing log level |
| `FRAGMENT_PORT` | `3000` | HTTP server port |
| `FRAGMENT_HOST` | `0.0.0.0` | HTTP server host |

### Production Checklist

- [ ] Use `--release` flag
- [ ] Set `RUST_LOG=warn` in production
- [ ] Configure a reverse proxy (nginx, traefik)
- [ ] Add health check: `#[route(GET, "/health")]`
- [ ] Enable CORS via `FragmentApp::new().cors_origins(...)`
- [ ] Wire up a real database (SQLx) instead of in-memory

### Health Check Endpoint

```rust
#[controller("/")]
impl HealthController {
    #[route(GET, "/health")]
    async fn health() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION")
        }))
    }
}
```

---

## Framework Architecture

```
fragment-rs/src/
  core/
    container/    # DI container, Inject, Service, auto-registration
    context/      # Request context & storage
    filter/       # ExceptionFilter trait
    http/         # FrgClient (HTTP client)
    metadata/     # MetadataStorage singleton
    privacy/      # Privacy/PII trait
    routing/      # FragmentRoute trait, FragmentRouter
    validation/   # ValidatedJson, ValidatedQuery
    workflow/     # WorkflowDef, WorkflowRunner, state store
  integrations/
    docs/         # OpenAPI spec builder
  platform/
    jobs/         # Job trait, JobScheduler
    web/          # FragmentApp, FragmentBootstrap
  plugins/        # Plugin trait, PluginManager
  shared/         # FragmentLogger, FragmentError, FragmentConfig, TTL
  types/          # ClassMetadata, MethodMetadata, ParamMetadata, etc.
```

### Auto-Registration Flow

```
compile time                    runtime (start())
    │                                  │
    ▼                                  ▼
#[service]  ────── injects ──▶  register_service(|| ...)
#[controller] ── injects ──▶  register_controller(|| ...)
                                 │
                                 ▼
                         DIContainer::auto_register_services()
                         DIContainer::auto_register_controllers()
                                 │
                                 ▼
                         FragmentApp wires middleware
                         axum::serve() binds TCP listener
```

### Request Flow

```
Request
  → Axum Router
    → FragmentRoute::register_routes()
      → Route handler
        → Inject<T> extractor (resolves from DIContainer)
        → ValidatedJson<T> extractor (validates body)
        → Handler logic
  → ExceptionFilter catches panics/errors
Response
```
