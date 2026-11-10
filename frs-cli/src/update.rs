//! `frs update` — replace the running binary with the latest release.
//!
//! The release assets are named after the platform they are built for, and
//! the name does not change between releases. That means the latest release
//! can be fetched without asking the API which release is the latest one.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository the releases live in.
const REPOSITORY: &str = "Jesse-Dan/fragment-rs";

/// Where the releases are fetched from.
///
/// `FRAGMENT_RELEASE_BASE` points the updater somewhere else, which is what
/// a mirror needs.
fn release_base() -> String {
    std::env::var("FRAGMENT_RELEASE_BASE")
        .ok()
        .map(|base| base.trim_end_matches('/').to_owned())
        .filter(|base| !base.is_empty())
        .unwrap_or_else(|| format!("https://github.com/{REPOSITORY}"))
}

/// The release asset built for this platform.
///
/// The names are stable across releases, so `releases/latest/download` finds
/// the right file without a lookup.
fn asset_name() -> Result<&'static str, String> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "frs-linux-x64",
        ("linux", "aarch64") => "frs-linux-arm64",
        ("macos", "x86_64") => "frs-macos-x64",
        ("macos", "aarch64") => "frs-macos-arm64",
        ("windows", "x86_64") => "frs-windows-x64",
        ("windows", "aarch64") => "frs-windows-arm64",
        (os, arch) => {
            return Err(format!(
                "there is no release for {os} on {arch}. \
                 Build it from source with `cargo install --git {REPOSITORY} frs-cli`."
            ))
        }
    })
}

/// The name of the binary inside the archive.
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "frs.exe"
    } else {
        "frs"
    }
}

/// The version of a binary, read by asking it.
fn version_of(binary: &Path) -> Result<String, String> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not run {}: {error}", binary.display()))?;
    if !output.status.success() {
        return Err("the downloaded binary did not run".to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .trim()
        .rsplit(' ')
        .next()
        .unwrap_or_default()
        .to_owned())
}

/// Downloads a file and returns where it was written.
fn download(url: &str, destination: &Path) -> Result<(), String> {
    let mut request = ureq::get(url);
    // A private repository needs a token. A public one does not.
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
    }

    let response = request.call().map_err(|error| match error {
        ureq::Error::Status(code, _) => {
            format!("the server answered {code} for {url}")
        }
        ureq::Error::Transport(transport) => {
            format!("could not reach {url}: {transport}")
        }
    })?;

    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)
        .map_err(|error| format!("could not write {}: {error}", destination.display()))?;
    std::io::copy(&mut reader, &mut file)
        .map_err(|error| format!("could not write {}: {error}", destination.display()))?;
    Ok(())
}

/// Unpacks the archive and returns the path to the binary inside it.
fn unpack(archive: &Path, destination: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(archive)
        .map_err(|error| format!("could not read {}: {error}", archive.display()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);

    tar.unpack(destination)
        .map_err(|error| format!("could not unpack {}: {error}", archive.display()))?;

    let binary = destination.join(binary_name());
    if !binary.exists() {
        return Err(format!("the archive did not contain {}", binary_name()));
    }
    Ok(binary)
}

/// Puts a downloaded binary in place of the running one.
///
/// On Unix the running binary can be replaced directly. Windows will not let
/// a running binary be overwritten, so it is moved aside first.
fn replace(current: &Path, replacement: &Path) -> Result<(), String> {
    if cfg!(windows) {
        let aside = current.with_extension("old");
        let _ = fs::remove_file(&aside);
        fs::rename(current, &aside)
            .map_err(|error| format!("could not move {}: {error}", current.display()))?;
        if let Err(error) = fs::rename(replacement, current) {
            // Put the old binary back, so the installation still works.
            let _ = fs::rename(&aside, current);
            return Err(format!(
                "could not put the new binary in place: {error}. \
                 Run the installer again to finish the update."
            ));
        }
        // The old binary cannot be deleted while it is running. Leaving it
        // behind is harmless; the next update removes it.
        let _ = fs::remove_file(&aside);
        return Ok(());
    }

    fs::rename(replacement, current).map_err(|error| {
        format!(
            "could not replace {}: {error}. \
             Run the installer again to finish the update.",
            current.display()
        )
    })
}

/// Runs `frs update`.
pub fn update(args: &[String]) -> Result<(), String> {
    let check_only = args.iter().any(|arg| arg == "--check" || arg == "-c");
    for arg in args {
        if arg != "--check" && arg != "-c" {
            return Err(format!(
                "unknown option `{arg}`. Usage: frs update [--check]"
            ));
        }
    }

    let asset = asset_name()?;
    let url = format!("{}/releases/latest/download/{asset}.tar.gz", release_base());

    println!("Checking for a newer release of frs...");

    let workdir = std::env::temp_dir().join(format!("frs-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&workdir);
    fs::create_dir_all(&workdir)
        .map_err(|error| format!("could not create {}: {error}", workdir.display()))?;

    let archive = workdir.join(format!("{asset}.tar.gz"));
    let result = (|| -> Result<(), String> {
        download(&url, &archive)?;

        let unpacked = workdir.join("unpacked");
        fs::create_dir_all(&unpacked)
            .map_err(|error| format!("could not create {}: {error}", unpacked.display()))?;
        let binary = unpack(&archive, &unpacked)?;

        make_executable(&binary)?;

        let latest = version_of(&binary)?;
        let current_version = env!("CARGO_PKG_VERSION");

        if latest == current_version {
            println!("frs {current_version} is the latest release.");
            return Ok(());
        }

        println!("Found frs {latest}. You have frs {current_version}.");

        if check_only {
            println!("Run `frs update` to install it.");
            return Ok(());
        }

        let current = std::env::current_exe()
            .map_err(|error| format!("could not find the running binary: {error}"))?;
        replace(&current, &binary)?;

        println!("Updated to frs {latest}.");
        Ok(())
    })();

    let _ = fs::remove_dir_all(&workdir);
    result
}

/// Makes the downloaded binary runnable.
fn make_executable(binary: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(binary)
            .map_err(|error| format!("could not read {}: {error}", binary.display()))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(binary, permissions)
            .map_err(|error| format!("could not make {} runnable: {error}", binary.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = binary;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{asset_name, binary_name, replace, version_of};
    use std::fs;
    use std::path::PathBuf;

    /// A unique temporary directory for one test.
    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "frs-update-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn there_is_an_asset_for_this_platform() {
        let asset = asset_name().unwrap();
        assert!(asset.starts_with("frs-"), "unexpected asset name {asset}");
        assert!(asset.ends_with("-x64") || asset.ends_with("-arm64"));
    }

    #[test]
    fn the_binary_name_matches_the_platform() {
        let expected = if cfg!(windows) { "frs.exe" } else { "frs" };
        assert_eq!(binary_name(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn the_version_is_read_from_the_binary() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("version");
        let script = dir.join("frs");
        fs::write(&script, "#!/bin/sh\necho 'frs 1.2.3'\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(version_of(&script).unwrap(), "1.2.3");
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_binary_that_does_not_run_is_an_error() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("broken");
        let script = dir.join("frs");
        fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(version_of(&script).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_running_binary_is_replaced() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("replace");
        let current = dir.join("frs");
        let replacement = dir.join("frs-new");
        fs::write(&current, "old").unwrap();
        fs::write(&replacement, "new").unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o755)).unwrap();

        replace(&current, &replacement).unwrap();

        assert_eq!(fs::read_to_string(&current).unwrap(), "new");
        assert!(!replacement.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
