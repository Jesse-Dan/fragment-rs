# Configuration

A Fragment application is configured by a `fragment.toml` file in the project
root, plus a small number of environment variables.

## fragment.toml

`frs create` writes a starting file. Every section is optional:

```toml
[structure]
type = "module"          # layered | module | feature

[server]
host = "0.0.0.0"
port = 3000
workers = 4

[databases.primary]
url = "postgres://postgres:postgres@localhost/fragment"
pool_size = 10

[databases.analytics]
url = "sqlite://analytics.db?mode=rwc"
pool_size = 2

[logging]
level = "info"
structured = false
file = "logs/fragment.log"

[privacy]
redact_logs = true
mask = "[REDACTED]"

[docs]
enabled = true
path = "/docs"
title = "My API"
version = "1.0.0"
```

### Sections

| Section | Purpose |
|---|---|
| `[structure]` | The project layout. Written by `frs create` and read by the generator. |
| `[server]` | Bind address and port for the HTTP server. |
| `[databases]` | Named database connections. See [ORM and databases](ORM.md). |
| `[logging]` | Log level, format, and optional log file. |
| `[privacy]` | Redaction rules for responses and logs. |
| `[cache]` | Cache driver, key prefix, and TTL. |
| `[rate_limit]` | Rate limit driver, prefix, request budget, and window. |
| `[workflows]` | Where workflow state is stored. |
| `[docs]` | OpenAPI document settings. |
| `verbose` | Top-level flag for verbose output. |

### Reading the configuration

`FragmentApp` reads the file for you:

```rust
use fragment_rs::FragmentApp;

FragmentApp::from_config_file("fragment.toml")?
    .start()
    .await;
```

The file is optional. When it is not there, the application starts with its
defaults, so a project runs before it has been configured. When it is there
but cannot be read or parsed, `from_config_file` returns the error rather
than ignoring it.

`from_config_file_or_default` ignores any problem with the file:

```rust
FragmentApp::from_config_file_or_default("fragment.toml")
    .start()
    .await;
```

That is what `frs create` writes into `src/main.rs`.

### Precedence

A setting can come from three places. The first one that has a value wins:

1. The builder method in the application code.
2. The environment variable.
3. The value in `fragment.toml`.
4. The built-in default.

```rust
// fragment.toml says port = 8080, FRAGMENT_PORT says 3001.
// The application listens on 3001.
FragmentApp::from_config_file("fragment.toml")?
    .start()
    .await;
```

The address can be checked without starting the server:

```rust
let app = FragmentApp::from_config_file("fragment.toml")?;
let (host, port) = app.address();
println!("will listen on http://{host}:{port}");
```

### Reading the file directly

The configuration can also be parsed on its own:

```rust
use fragment_rs::FragmentConfig;

let config = FragmentConfig::load("fragment.toml")?;

if let Some(server) = &config.server {
    println!("listening on {:?}:{:?}", server.host, server.port);
}

for (name, database) in &config.databases {
    println!("{name}: {:?}", database.url);
}
```

`FragmentConfig::load` returns a `ConfigError` when the file cannot be read or
parsed:

```rust
match FragmentConfig::load("fragment.toml") {
    Ok(config) => println!("{:?}", config.server),
    Err(error) => eprintln!("could not read the configuration: {error}"),
}
```

`FragmentConfig::from_env()` builds a configuration from environment variables
alone, and `FragmentConfig::load_defaults()` returns an empty configuration.

## Environment variables

| Variable | Read by | Description |
|---|---|---|
| `DATABASE_URL` | migration CLI, `frs make:migration -g` | Connection URL for the database. |
| `FRAGMENT_HOST` | `FragmentApp` | Override the bind address. |
| `FRAGMENT_PORT` | `FragmentApp` | Override the port. |
| `FRAGMENT_LOG_LEVEL` | `FragmentConfig::from_env` | Log level. |
| `FRAGMENT_LOG_STRUCTURED` | `FragmentConfig::from_env` | `true` for structured logs. |
| `FRAGMENT_VERBOSE` | `FragmentConfig::from_env` | `true` for verbose output. |
| `RUST_LOG` | tracing subscriber | Standard filter, for example `info,fragment_rs=debug`. |

The Rust `RUST_LOG` variable takes precedence over the `logging.level` in
`fragment.toml` and over `FRAGMENT_LOG_LEVEL`, because the logger reads it
directly.

## Server settings

```rust
use fragment_rs::FragmentApp;

FragmentApp::new()
    .host("0.0.0.0")
    .port(3000)
    .cors_origins(vec!["http://localhost:5173".into()])
    .cors_credentials(true)
    .start()
    .await;
```

| Setting | Builder method | Default |
|---|---|---|
| Bind address | `host` | `0.0.0.0` |
| Port | `port` | `3000` |
| Allowed origins | `cors_origins` | CORS disabled |
| Credentials | `cors_credentials` | `false` |
| File upload size | `file_upload_max_size` | 10 MiB |
| File upload types | `file_upload_allowed_types` | any type |
| File upload count | `file_upload_max_files` | 10 |
| Log level | `log_level` | `info` |

## Logging

The logger installs a `tracing` subscriber on startup and reads `RUST_LOG`:

```sh
RUST_LOG=info,fragment_rs=debug cargo run
```

`FragmentLogger::init` is called by `FragmentApp::start` and by
`FragmentRouter::start`, so applications do not have to configure logging
themselves. Calling it twice is safe.

## Privacy and redaction

`FragmentPrivacyService` masks sensitive values before they are returned to a
client or written to a log:

```rust
use fragment_rs::core::privacy::FragmentPrivacyService;
use fragment_rs::shared::config::PrivacyConfig;

let config = PrivacyConfig {
    redact_logs: Some(true),
    encryption_key: None,
    mask: Some("[REDACTED]".into()),
};

let masked = FragmentPrivacyService::sanitize_object(&value, &config);
```

The default key list covers `password`, `token`, `secret`, `ssn`,
`credit_card`, `card_number`, and `cvv`. Set `FRAGMENT_REDACT_KEYS` to a
comma-separated list to replace it.

## See also

- [ORM and databases](ORM.md) — named connections and migrations
- [CLI reference](CLI.md) — the commands that read this configuration
- [Framework usage guide](../USAGE.MD) — services, controllers, and validation
