# Development and releases

How to work on Fragment Rust itself, how to test it, and how a release is
published.

## Repository layout

```text
.
├── fragment-rs/          the framework crate
├── fragment-macros/      the procedural macros
├── frs-cli/              the `frs` command-line tool
├── docs/                 the documentation
├── scripts/              the installers, the release script, the publish script
└── .github/workflows/    the CI and release pipelines
```

| Crate | Publishes | Purpose |
|---|---|---|
| `fragment-rs` | `fragment-rs` | The framework: DI, routing, validation, errors, ORM, HTTP client. |
| `fragment-macros` | `fragment-macros` | The `#[controller]`, `#[route]`, `#[service]`, and `#[injectable]` macros. |
| `frs-cli` | `frs-cli` | The `frs` binary that scaffolds projects. |

## Prerequisites

- A stable Rust toolchain: <https://rustup.rs>
- `cargo clippy` and `cargo fmt`, which ship with the toolchain

```sh
rustup component add clippy rustfmt
```

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### What the tests cover

| Area | Location |
|---|---|
| Framework errors, database connections, and ORM round trips | `fragment-rs/src/**` |
| Name conversion, scaffolding, module generation, entity generation, migration generation | `frs-cli/src/main.rs` |
| Entity parsing | `frs-cli/src/entities.rs` |
| Schema diffing and migration rendering | `frs-cli/src/schema.rs` |

The CLI tests create real projects in a temporary directory, and the migration
tests run against a real SQLite database, so they exercise the same code path
a user does.

### Running the examples

```sh
cargo run -p fragment-rs --example orm_entities
cargo run -p fragment-rs --example multi_database
cargo run -p fragment-rs --example customers_api
cargo run -p fragment-rs --example quickstart_demo
```

## Working on the CLI

The CLI is a normal workspace member:

```sh
cargo build -p frs-cli
cargo run -p frs-cli -- help
cargo install --path frs-cli --locked
```

`cargo install --path frs-cli --locked` puts `frs` on your `PATH`. Use
`--force` to replace an installed copy.

To try a change against a scratch project:

```sh
frs create scratch --structure module
cd scratch
frs make:mod authentication
frs make:controller login --pmod=authentication
frs check
```

## Working on the macros

The macros live in `fragment-macros`. `#[controller]` expands an `impl` block
into an Axum router registration:

```rust
#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list() -> Json<Vec<Customer>> { /* ... */ }
}
```

The prefix and the route path are joined without a trailing slash, so
`#[route(GET, "/")]` under the prefix `/api/customers` registers
`/api/customers`. The method constant is read from the attribute and matched
against `GET`, `POST`, `PUT`, `DELETE`, and `PATCH`; an unknown method is a
compile error.

## Releasing

```sh
./scripts/release.sh 0.3.0      # bump the version, tag, push, and publish
./scripts/release.sh --dry-run  # show what would happen
```

The script bumps the version in every crate, refreshes `Cargo.lock`, commits,
creates an annotated tag, and pushes both. Pushing the tag starts the
workflow in `.github/workflows/release.yml`, which:

1. runs `cargo test --workspace --locked`,
2. builds `frs` for Linux x86_64, macOS x86_64, macOS arm64, and Windows x86_64,
3. publishes the archives and a `SHA256SUMS` file as a GitHub release.

The script refuses to run when the working tree has changes or when the tag
already exists. Pass `--no-publish` to skip the crates.io publish.

### Publishing to crates.io

The crates are published in dependency order: the macros first, then the
framework, then the CLI. crates.io needs a moment to index a crate before
another crate can depend on it, so each step waits.

Put the token in `.env` in the project root. The file is ignored by git, so
the token is never committed:

```sh
cp .env.example .env
$EDITOR .env          # CARGO_REGISTRY_TOKEN=...
```

Then release as usual. The release script publishes the crates after the tag
is pushed:

```sh
./scripts/release.sh 0.3.0
```

To publish without tagging, run the publish script on its own:

```sh
./scripts/publish.sh
./scripts/publish.sh --dry-run
```

The same token can be set as the `CARGO_REGISTRY_TOKEN` secret in the
repository. The workflow then publishes the crates as well, in the same
order. That job is skipped when the secret is not set, so a fork can still
cut a release.

The token needs the `publish-new-crate` scope the first time a crate is
published, and `publish` after that. Create one at
<https://crates.io/settings/tokens>.

### Before tagging

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Installing a released CLI

```sh
curl -fsSL "https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh" \
  | sh -s -- --repo Jesse-Dan/fragment-rs
```

See [Installation](INSTALL.md) for the Windows instructions and the custom
install directory options.

## Reporting problems

Open an issue with:

- the `frs --version` output,
- the Rust toolchain version from `rustc --version`,
- the structure the project uses,
- the command you ran and what it printed.

For a bug in the framework itself, a failing test is the most useful thing you
can include.

## See also

- [Installation](INSTALL.md)
- [CLI reference](CLI.md)
- [Framework usage guide](../USAGE.MD)
