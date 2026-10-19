use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "soroban-guard-{label}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn nonexistent_scan_path_exits_two_with_clear_message() {
    let root = temporary_directory("missing-path");
    let missing = root.join("does-not-exist");
    let output = Command::new(env!("CARGO_BIN_EXE_soroban-guard"))
        .args(["scan", missing.to_str().unwrap()])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains(&missing.display().to_string()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_path_is_used_when_scan_argument_is_absent() {
    let root = temporary_directory("config-path");
    std::fs::create_dir_all(root.join("contract/src")).unwrap();
    std::fs::write(root.join("contract/src/lib.rs"), "pub fn harmless() {}\n").unwrap();
    std::fs::write(root.join("soroban-guard.toml"), "[scan]\npath = \"contract\"\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_soroban-guard"))
        .arg("scan")
        .current_dir(&root)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(root).unwrap();
}

/// Issues #626 / #726: a malformed `./soroban-guard.toml` must not abort a scan
/// of an unrelated, explicitly given path.
#[test]
fn malformed_cwd_config_does_not_abort_scan_of_explicit_path() {
    let cwd = temporary_directory("malformed-cwd-config");
    std::fs::write(cwd.join("soroban-guard.toml"), "[scan\nthis is not toml = =\n").unwrap();

    let target = temporary_directory("explicit-target");
    std::fs::create_dir_all(target.join("src")).unwrap();
    std::fs::write(target.join("src/lib.rs"), "pub fn harmless() {}\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_soroban-guard"))
        .args(["scan", target.to_str().unwrap()])
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(cwd).unwrap();
    std::fs::remove_dir_all(target).unwrap();
}

/// The fallback still reports a malformed cwd config when no path was given,
/// because then the config is actually needed.
#[test]
fn malformed_cwd_config_still_errors_when_no_path_is_given() {
    let cwd = temporary_directory("malformed-cwd-config-nopath");
    std::fs::write(cwd.join("soroban-guard.toml"), "[scan\nthis is not toml = =\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_soroban-guard"))
        .arg("scan")
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::remove_dir_all(cwd).unwrap();
}
