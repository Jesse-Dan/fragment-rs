# frs-cli

The command-line tool for the [Fragment Rust](https://github.com/Jesse-Dan/fragment-rs) framework.

`frs` creates an application, generates the modules and components inside it,
writes migrations, and keeps itself up to date.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

On Windows:

```powershell
irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
```

The installer puts the binary in `~/.local/bin` and adds it to the `PATH` in
your shell profile.

## Commands

| Command | Purpose |
|---|---|
| `frs create <name>` | Create an application. `--structure layered\|module\|feature`. |
| `frs make:mod <name>` | Add a domain module. `--pmod <parent>`. |
| `frs make:controller <name>` | Add a controller. |
| `frs make:service <name>` | Add a service. |
| `frs make:entity <name>` | Add a SeaORM entity. |
| `frs make:migration <name>` | Write a migration. `-g` generates it from the entities and the database. |
| `frs check` | Run `cargo check` in the project. |
| `frs doctor` | Check the Rust toolchain. |
| `frs update` | Update the CLI to the latest release. `--check` only reports. |
| `frs version` | Show the version. |

## Examples

```sh
frs create storefront --structure module
cd storefront
frs make:mod authentication
frs make:controller login --pmod=authentication
frs make:entity user
frs make:migration create_users_table
frs make:migration -g --database-url postgres://localhost/app
```

## `frs update`

`frs update` replaces the running binary with the latest release:

```sh
frs update          # install the latest release
frs update --check  # report whether there is a newer release
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option.
