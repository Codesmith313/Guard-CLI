use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn init_writes_exactly_one_trailing_newline() {
    let root = PathBuf::from(std::env::temp_dir()).join(format!(
        "soroban-guard-init-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_soroban-guard"))
        .args(["init", root.to_str().unwrap()])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let contents = std::fs::read_to_string(root.join("soroban-guard.toml")).unwrap();
    assert!(contents.ends_with("extra = []\n"));
    assert!(!contents.ends_with("\n\n"));
    std::fs::remove_dir_all(root).unwrap();
}
