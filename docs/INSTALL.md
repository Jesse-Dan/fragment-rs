# Install the `frs` CLI

`frs` is the Fragment Rust command-line tool. It creates applications,
scaffolds modules, and generates source-file starting points.

## Install

### From crates.io

```sh
cargo install frs-cli
```

This is the usual way to install a Rust tool. It builds from source, so it
needs a Rust toolchain, and it puts `frs` in `~/.cargo/bin`.

### From a release

#### macOS and Linux

```sh
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

#### Windows

```powershell
irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
```

The installer downloads the latest release, unpacks it, and puts `frs` in
`~/.local/bin`. It then adds that directory to your `PATH` in your shell
profile, so `frs` runs from anywhere. Open a new shell afterwards, or run:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

The release binary is ready to run, with no build step.

### Update

```sh
frs update
```

That replaces the running binary with the latest release. `frs update --check`
reports the version it would install and changes nothing.

### Other directories

```sh
FRS_INSTALL_DIR="$HOME/tools" \
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

```powershell
$env:FRS_INSTALL_DIR = "$HOME\tools"
irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
```

### A different repository

```sh
FRS_REPO="owner/repository" \
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

### A private repository

A private repository needs a token to download its release assets. Set
`GITHUB_TOKEN` and the installer uses it:

```sh
GITHUB_TOKEN=ghp_xxx \
curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
```

```powershell
$env:GITHUB_TOKEN = "ghp_xxx"
irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
```

The token needs read access to the repository.

## Supported platforms

| Operating system | Architecture | Release asset |
|---|---|---|
| Linux | x86_64 | `frs-linux-x64.tar.gz` |
| macOS | Intel | `frs-macos-x64.tar.gz` |
| macOS | Apple Silicon | `frs-macos-arm64.tar.gz` |
| Windows | x86_64 | `frs-windows-x64.zip` |

Every release also publishes a `frs-windows-x64.tar.gz`, so `frs update` can
unpack every platform the same way.

Other platforms can install from source.

## Install from source

Install Rust with [rustup](https://rustup.rs), clone this repository, then run:

```sh
cargo install --path frs-cli --locked
frs --version
```

To replace an installed copy:

```sh
cargo install --path frs-cli --locked --force
```

Or install straight from the repository, without cloning it:

```sh
cargo install --git https://github.com/Jesse-Dan/fragment-rs frs-cli --locked
```

## Verify the installation

```sh
frs --version
frs help
frs doctor
```

`frs doctor` reports whether `rustc` and `cargo` are available. The CLI uses
Cargo to build generated projects, so install Rust even when using a prebuilt
`frs` binary.

## Uninstall

```sh
rm -f "$HOME/.local/bin/frs"
```

```powershell
Remove-Item "$HOME\.local\bin\frs.exe"
```

## Checking a download

Every release publishes a `SHA256SUMS` file. To verify an archive yourself:

```sh
sha256sum frs-linux-x64.tar.gz
cat SHA256SUMS
```

The two hashes must match. Checksums detect corruption and incomplete
downloads; they do not replace trusting the source of the release.
