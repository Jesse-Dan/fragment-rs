# fragment-rs

A Spring Boot-inspired web framework for Rust, built on [Axum](https://github.com/tokio-rs/axum).

Fragment gives you the parts a web service usually needs, without the wiring:

- **Controllers that register themselves.** `#[controller("/api/customers")]`
  adds its routes and the application finds it.
- **Dependency injection.** `#[service]` puts a service in the container and
  handlers ask for it with `Inject`.
- **Validation.** `ValidatedJson` and `ValidatedQuery` reject bad input with
  a readable error before the handler runs.
- **Structured errors.** Every failure answers with the same JSON envelope,
  and internal details never reach the client.
- **Configuration.** `fragment.toml` sets the address, the log level, and the
  named database connections.
- **An ORM.** SeaORM entities, ActiveModels, relationships, and queries, with
  migrations for SQLite, PostgreSQL, and MySQL.

## Getting started

```sh
cargo install frs-cli
frs create my-service
cd my-service
cargo run
```

The generated application has no wiring code. Add a controller and a service,
and they are found at startup.

## A first controller

```rust,ignore
use fragment_rs::{controller, route, service, GET, POST};

#[service]
#[derive(Default)]
struct CustomerRepository {
    customers: Vec<Customer>,
}

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepository>) -> Json<Vec<Customer>> {
        Json(repo.customers.clone())
    }

    #[route(POST, "/")]
    async fn create(
        Inject(repo): Inject<CustomerRepository>,
        ValidatedJson(body): ValidatedJson<CreateCustomer>,
    ) -> (StatusCode, Json<Customer>) {
        let customer = Customer::from(body);
        repo.customers.push(customer.clone());
        (StatusCode::CREATED, Json(customer))
    }
}
```

## Configuration

```toml
[server]
host = "0.0.0.0"
port = 3000

[logging]
level = "info"

[databases.primary]
url = "postgres://postgres:postgres@localhost/fragment"
```

`FRAGMENT_HOST` and `FRAGMENT_PORT` override the file, and `RUST_LOG` overrides
the log level.

## Errors

Every error answers with the same shape:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed: name: length",
    "details": null
  }
}
```

A client error says what went wrong. A server error says only that something
went wrong; the details stay in the log.

## Examples

```sh
cargo run --example quickstart_demo
cargo run --example customers_api
cargo run --example multi_database
cargo run --example orm_entities
```

## Documentation

- [Framework usage guide](https://github.com/Jesse-Dan/fragment-rs/blob/main/USAGE.MD)
- [Configuration](https://github.com/Jesse-Dan/fragment-rs/blob/main/docs/CONFIGURATION.md)
- [ORM and databases](https://github.com/Jesse-Dan/fragment-rs/blob/main/docs/ORM.md)
- [CLI reference](https://github.com/Jesse-Dan/fragment-rs/blob/main/docs/CLI.md)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option.
