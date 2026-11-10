# Project structure modes

The layout is part of a generated app's architecture. `frs create` writes the selected mode into `fragment.toml`; generation commands read that setting to decide where files go.

```toml
[structure]
type = "module" # layered | module | feature
```

Select a layout when creating an app:

```sh
frs create example-layered --structure layered
frs create example-modules --structure module
frs create example-features --structure feature
```

The default is `module`. The current CLI does not yet provide an in-place `frs structure` conversion command. To change layouts, create a new project with the desired mode and move/refactor source deliberately; avoid changing only `fragment.toml`, as existing files will not be reorganized automatically.

## Layered

Choose layered mode when code is organized by technical responsibility and each layer is shared across the application.

```text
src/
├── main.rs
├── controllers/
│   ├── mod.rs
│   └── app_controller.rs
├── services/
│   ├── mod.rs
│   └── app_service.rs
├── repositories/
│   └── mod.rs
├── entities/
│   └── mod.rs
├── dto/
│   └── mod.rs
└── common/
    └── mod.rs
```

`frs make:controller <name>` writes to `src/controllers/`; `frs make:service <name>` writes to `src/services/`; `frs make:entity <name>` writes to `src/entities/`. `frs make:mod` is not supported for this mode because domain-grouped modules conflict with the layer-first organization.

Migrations live in their own crate outside `src/`, so no layout has a `migrations` directory in the source tree.

## Module

Choose module mode when each domain owns its controller and service code.

```text
src/
├── main.rs
├── modules/
│   ├── mod.rs
│   └── app/
│       ├── mod.rs
│       ├── controllers/
│       │   ├── mod.rs
│       │   └── app_controller.rs
│       └── services/
│           ├── mod.rs
│           └── app_service.rs
└── entities/
    └── mod.rs
```

Generate and nest modules:

```sh
frs make:mod authentication
frs make:mod providers --pmod=authentication
```

This creates `src/modules/authentication/` and `src/modules/authentication/providers/`, each with a `mod.rs`, `controllers/`, and `services/`. The parent `mod.rs` files are updated to declare generated child modules. `--pmod` accepts slash-separated nested paths, such as `--pmod=api/auth`.

## Feature

Choose feature mode when source is organized around business capabilities, with shared platform concerns separated from features.

```text
src/
├── main.rs
├── features/
│   ├── mod.rs
│   └── app/
│       ├── mod.rs
│       ├── controllers/
│       └── services/
├── shared/
│   ├── mod.rs
│   ├── guards/
│   ├── filters/
│   └── middleware/
└── entities/
```

`frs make:mod authentication` generates `src/features/authentication/`; nested modules use `--pmod` as in module mode.

## Parent module behavior

```sh
frs make:mod authentication --pmod=api/v1
```

When a parent path does not exist, the CLI creates the missing parent module scaffolding and updates the Rust module declarations. It does not overwrite an existing module with the same name. Review the generated `mod.rs` files after a failed or interrupted generation before rerunning.

## Rust module declarations

The generator emits `mod.rs` declarations and updates `src/main.rs`. These files are ordinary Rust modules: keep module declarations synchronized if you later move or rename generated files. Run `frs check` after generation to catch missing declarations or syntax errors.

## Choosing a mode

| Mode | Organizes by | Good fit when |
|---|---|---|
| `layered` | Technical role | Layers are shared and the application is small or conventional |
| `module` | Domain/module | Domains own their controllers and services |
| `feature` | Business capability | Teams ship features independently and shared infrastructure stays small |

All modes are templates, not automatic scanners or dependency-injection modules. The CLI generates source structure; Rust module inclusion and runtime wiring remain explicit.
