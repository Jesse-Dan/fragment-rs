# fragment-macros

Procedural macros for the [Fragment Rust](https://github.com/Jesse-Dan/fragment-rs) framework.

This crate is an implementation detail of `fragment-rs`. Use the framework crate
rather than depending on this one directly.

## What it provides

| Attribute | Purpose |
|---|---|
| `#[controller("/prefix")]` | Registers a controller's routes and makes the application find it. |
| `#[service]` | Registers a service with the dependency injection container. |
| `#[route(GET, "/")]` | Declares a route on a controller method. |
| `#[derive(FragmentPrivacy)]` | Marks fields for redaction. |

## Usage

```rust,ignore
use fragment_rs::{controller, route, service, GET};

#[service]
#[derive(Default)]
struct CustomerRepository {
    customers: Vec<String>,
}

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list() -> Vec<String> {
        Vec::new()
    }
}
```

The controller and the service register themselves, so an application does not
have to wire them up. See the [framework documentation](https://github.com/Jesse-Dan/fragment-rs)
for the full guide.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option.
