# fragment-rs Roadmap

## Fixes — API that lies (highest priority)
- Implement `.helmet(true)` — add security headers (tower-http `SetResponseHeaderLayer` is already a dependency)
- Implement `.compression(true)` — add `CompressionLayer`
- Implement `.auto_scan(false)` — actually honour it, or delete it
- Implement `.file_upload_max_size` / `allowed_types` / `max_files` — enforce with `DefaultBodyLimit` + multipart handling
- Remove "OpenAPI docs" from the crate description, or wire `OpenApiBuilder` to introspect the registered controllers
- Wire the `[docs]` section of `fragment.toml` to serve `/openapi.json`
- Implement `ExceptionFilter::catch` as real middleware, or delete the trait and `#[exception_filter]` macro
- Make `#[derive(FragmentPrivacy)]` + `#[pii]` actually redact the marked fields
- Make `#[service]` emit a compile error or runtime warning when it cannot self-register

## Fixes — correctness
- Make `RequestContextStorage` task-local instead of a global static (currently leaks across concurrent requests)
- Make `#[service]` detect a manual `impl Default`, not just the derive
- Consolidate `#[service]` and `#[injectable]` into one mechanism
- Have `FragmentApp` use `FragmentConfig::from_env()` so env vars have one source of truth
- Either implement `FragmentBootstrap` properly or delete it (currently only logs)
- Decide the fate of `FragmentRouter` — two entry points alongside `FragmentApp` is confusing
- Check `WorkflowRunner`, `FrgClient`, and the jobs module are real, not stubs

## Fixes — unimplemented config sections
- `[cache]` — parsed but never read; nothing implements it
- `[workflows]` — parsed but never read
- `[rate_limit]` — 4 references; verify it is actually enforced per-request
- `[structure]` — parsed but unused at runtime (only the CLI reads it)

## Features
- Hot reload — add `frs run` that uses `cargo watch`; the single biggest gap versus Spring Boot
- A test harness — `@WebMvcTest` / `@SpringBootTest` equivalent, with a `TestApp` that builds the router for you
- OpenAPI generation by introspecting controllers and their `#[route]` attributes
- Auto-generate the client project from the OpenAPI spec (`frs make:client`)
- Constructor injection, to match Spring more closely
- Profile-based config (`[profiles.dev]`, `[profiles.prod]`)
- Graceful shutdown — drain in-flight requests on SIGTERM
- Health and readiness endpoints built in
- `frs new` interactive mode, for people who don't know the structure options
- Rate limiting, caching, and sessions as real middleware layers

## Developer experience
- Warn at startup when a builder method has no effect
- Print the resolved configuration at startup (host, port, log level, databases)
- Better error messages when a route collides or a service is missing
- Rust-analyzer support — the macros should produce useful completions; inventory registration is invisible to the IDE
- `cargo xtask`-style single command for fmt + clippy + test + publish
- A `fragment doctor --deep` that checks the toolchain, the database, the config file, and the project structure

## Docs
- A "what's implemented vs. planned" page — honest status per feature
- Migration guides between versions
- A cookbook: auth, file upload, background jobs, websockets
- Document the precedence rules (builder → env → file → default) in one place
- A troubleshooting page for the common failures

## Release and infrastructure
- Publish `fragment-rs` and `frs-cli` to crates.io (in progress — macros are up)
- Add a CI job that runs `cargo publish --dry-run` on every PR
- Add `cargo clippy -- -D warnings` to CI
- Add MSRV to the manifests and check it in CI
- Add a dependabot config
- Sign the release artifacts and publish a checksum file per artifact
- Add a `CHANGELOG.md` kept by hand

## Cleanup
- Delete `fragment-rs/src/types/decoration_types.rs` — clippy flags it as a derivable impl
- Fix the clippy warnings in `entities.rs` and `container/mod.rs`
- Remove `FragmentConfig::load_defaults()` if it has no callers
- Move `FragmentError::Generic`'s code from `E{status}` to meaningful slugs

## Suggested order
1. **Now:** items 1–4 (implement the lying builders), correctness items 9 and 10, publish to crates.io (item 2)
2. **Next:** hot reload (21), OpenAPI (5 and 23), test harness (22)
3. **Then:** consolidate the API (11–16), implement or remove the config sections (17–20), delete or implement the dead macros (7 and 8)
4. **Later:** docs, cleanup, and CI hardening

**The pattern to follow:** anything in the public API either works or doesn't exist. Everything in the first group is about that.