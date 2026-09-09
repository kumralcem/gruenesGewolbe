use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_captures_manual_fallback_text_into_a_specified_vault() {
    let root = temp_path("cli-capture-vault");

    let create = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("create")
        .arg(&root)
        .output()
        .expect("run create command");
    assert!(create.status.success());

    let capture = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("capture-manual-text")
        .arg(&root)
        .arg("https://example.com/nocturne-note")
        .arg("Nocturne note")
        .arg("Useful framing for night palettes")
        .arg("Main idea text from the page.")
        .output()
        .expect("run capture command");
    assert!(capture.status.success());

    let stdout = String::from_utf8(capture.stdout).expect("capture stdout");
    assert!(stdout.starts_with("captured-manual-text\t"));
    assert!(stdout.contains("\tIdea Sources\t"));

    let results = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("search")
        .arg(&root)
        .arg("night palettes")
        .output()
        .expect("run search command");
    assert!(results.status.success());
    assert!(String::from_utf8(results.stdout)
        .expect("search stdout")
        .contains("Nocturne note"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
