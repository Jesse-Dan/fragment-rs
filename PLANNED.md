# Planned work

The framework is at 0.2.5. This is what is still outstanding, in the order
it should be done.

## 1. The API that lies

These builder methods store a value and nothing reads it. They have to either
work or not exist. A method that silently does nothing is worse than a
missing one, because a caller believes it took effect.

| Call | What happens now | What it should do |
|---|---|---|
| `.helmet(true)` | nothing | Add the security headers |
| `.compression(true)` | nothing | Add the compression layer |
| `.auto_scan(false)` | nothing | Honour it, or remove it |
| `.file_upload_max_size(…)` | nothing | Enforce the limit |
| `.file_upload_allowed_types(…)` | nothing | Enforce the types |
| `.file_upload_max_files(…)` | nothing | Enforce the count |

`tower-http` is already a dependency, so each of these is a few lines.

## 2. Dead and misleading API

- `ExceptionFilter` and `#[exception_filter]` are never called by anything.
  Implement them as middleware or delete them.
- `#[derive(FragmentPrivacy)]` and `#[pii]` are parsed and then ignored.
  Redaction is key-name substring matching today.
- `#[injectable]` is a second way to do what `#[service]` does, and it does
  not register anything. Fold it into `#[service]`.
- The crate description claims "OpenAPI docs". `OpenApiBuilder` is a manual
  builder; nothing introspects the controllers, and the `[docs]` section of
  `fragment.toml` is not wired to anything. Either build it or drop the
  claim.
- `FragmentBootstrap` only logs. Implement it or delete it.
- `FragmentRouter` and `FragmentApp` are two entry points. Pick one.

## 3. Correctness

- `RequestContextStorage` is a global static, so one request can see
  another's context. It needs to be task-local.
- `#[service]` self-registers only when the type derives `Default`. A
  hand-written `impl Default` is invisible to the macro, so the service
  silently never registers. It needs to fail loudly.
- `FragmentConfig::from_env()` and `FragmentApp` read the environment
  separately. One source of truth.

## 4. Configuration that is parsed but unused

`[cache]`, `[workflows]`, `[structure]` and `[docs]` are read into the config
and then never used at runtime. Implement them or remove them from the file
format.

## 5. Features worth having

- **Hot reload.** A `frs run` that watches the sources and restarts. This is
  the biggest gap against Spring Boot, whose whole promise is a fast
  restart loop.
- **A test harness.** Something like `@WebMvcTest` that builds the router
  for you, so a test does not have to wire routes by hand.
- **OpenAPI by introspection**, then `frs make:client` to generate a client
  from it.
- **Constructor injection**, to match Spring more closely.
- **Profile-based configuration**, for example `[profiles.dev]`.
- **Graceful shutdown**, so in-flight requests finish on SIGTERM.
- **Health and readiness endpoints.**
- **Rate limiting and caching** as real middleware.

## 6. Developer experience

- Warn at startup when a builder method had no effect.
- Print the resolved configuration at startup.
- Better errors when two routes collide or a service is missing.
- Make the macros produce useful completions. Registration through
  `inventory` is invisible to rust-analyzer.
- One command that runs fmt, clippy, tests and publish.

## 7. Documentation

- A page that says plainly what is implemented and what is planned.
- Migration guides between versions.
- A cookbook: authentication, file uploads, background jobs, websockets.
- The precedence rules (builder, then environment, then file, then default)
  written down once.

## 8. Infrastructure

- A CI job that runs `cargo publish --dry-run` on every pull request.
- `cargo clippy -- -D warnings` in CI.
- A minimum supported Rust version, checked in CI.
- A `CHANGELOG.md`.

## 9. Cleanup

- `fragment-rs/src/types/decoration_types.rs` — the impls can be derived.
- The clippy warnings in `entities.rs` and `core/container/mod.rs`.
- `FragmentConfig::load_defaults()` if nothing calls it.
- `FragmentError::Generic` builds its code as `E{status}`. Real slugs read
  better.
