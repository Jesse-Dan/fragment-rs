//! Checks that `fragment.toml` configures the application.
//!
//! The environment variables `FRAGMENT_HOST` and `FRAGMENT_PORT` override the
//! file, so they are cleared here. The tests for the environment live in
//! `config_env.rs`, which runs in its own process.

use fragment_rs::FragmentApp;

/// Removes the environment overrides, so the file is what decides.
fn clear_environment() {
    std::env::remove_var("FRAGMENT_HOST");
    std::env::remove_var("FRAGMENT_PORT");
    std::env::remove_var("FRAGMENT_LOG_LEVEL");
}

/// Writes a configuration file and returns the directory holding it.
fn write_config(body: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("fragment.toml"), body).unwrap();
    dir
}

#[test]
fn the_server_section_sets_the_address() {
    clear_environment();
    let dir = write_config("[server]\nhost = \"127.0.0.1\"\nport = 8080\n");

    let app = FragmentApp::from_config_file(dir.path().join("fragment.toml")).unwrap();

    assert_eq!(app.address(), ("127.0.0.1".to_owned(), 8080));
}

#[test]
fn a_missing_file_leaves_the_defaults_in_place() {
    clear_environment();
    let dir = tempfile::TempDir::new().unwrap();

    let app = FragmentApp::from_config_file(dir.path().join("fragment.toml")).unwrap();

    assert_eq!(app.address(), ("0.0.0.0".to_owned(), 3000));
}

#[test]
fn a_broken_file_is_reported() {
    clear_environment();
    let dir = write_config("[server\nport = 8080\n");

    let result = FragmentApp::from_config_file(dir.path().join("fragment.toml"));

    assert!(result.is_err(), "a broken file should be reported");
}

#[test]
fn a_broken_file_does_not_stop_the_application_from_starting() {
    clear_environment();
    let dir = write_config("[server\nport = 8080\n");

    let app = FragmentApp::from_config_file_or_default(dir.path().join("fragment.toml"));

    assert_eq!(app.address(), ("0.0.0.0".to_owned(), 3000));
}

#[test]
fn an_explicit_address_wins_over_the_file() {
    clear_environment();
    let dir = write_config("[server]\nhost = \"127.0.0.1\"\nport = 8080\n");

    let app = FragmentApp::from_config_file(dir.path().join("fragment.toml"))
        .unwrap()
        .host("0.0.0.0")
        .port(9090);

    assert_eq!(app.address(), ("0.0.0.0".to_owned(), 9090));
}

#[test]
fn the_log_level_is_read_from_the_file() {
    clear_environment();
    let dir = write_config("[logging]\nlevel = \"debug\"\n");

    let app = FragmentApp::from_config_file(dir.path().join("fragment.toml")).unwrap();

    // The level is stored, so the logger can use it when `RUST_LOG` is not
    // set.
    assert_eq!(app.configured_log_level().as_deref(), Some("debug"));
}
