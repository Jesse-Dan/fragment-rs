//! Checks that the environment overrides the configuration file.
//!
//! Environment variables are process-wide, so this test lives in its own
//! file and runs in its own process.

use fragment_rs::FragmentApp;

#[test]
fn the_environment_overrides_the_file() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("fragment.toml"),
        "[server]\nhost = \"127.0.0.1\"\nport = 8080\n",
    )
    .unwrap();

    let app = FragmentApp::from_config_file(dir.path().join("fragment.toml")).unwrap();
    assert_eq!(app.address(), ("127.0.0.1".to_owned(), 8080));

    std::env::set_var("FRAGMENT_HOST", "10.0.0.1");
    std::env::set_var("FRAGMENT_PORT", "3001");

    assert_eq!(app.address(), ("10.0.0.1".to_owned(), 3001));

    std::env::remove_var("FRAGMENT_HOST");
    std::env::remove_var("FRAGMENT_PORT");
}
