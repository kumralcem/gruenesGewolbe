use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_validates_whether_a_folder_is_a_usable_vault() {
    let root = temp_path("cli-validate");

    let create = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("create")
        .arg(&root)
        .output()
        .expect("run create command");
    assert!(create.status.success());
    assert_eq!(
        String::from_utf8(create.stdout).expect("create stdout"),
        format!("created\t{}\n", root.display())
    );

    let validate = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("validate")
        .arg(&root)
        .output()
        .expect("run validate command");
    assert!(validate.status.success());
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate stdout"),
        format!("valid\t{}\n", root.display())
    );

    let open = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("open")
        .arg(&root)
        .output()
        .expect("run open command");
    assert!(open.status.success());
    assert_eq!(
        String::from_utf8(open.stdout).expect("open stdout"),
        format!("opened\t{}\n", root.display())
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn cli_reports_clear_errors_for_invalid_vault_folders() {
    let root = temp_path("cli-invalid");
    fs::create_dir_all(&root).expect("create invalid root");

    let validate = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("validate")
        .arg(&root)
        .output()
        .expect("run validate command");

    assert!(!validate.status.success());
    assert_eq!(
        String::from_utf8(validate.stderr).expect("validate stderr"),
        format!(
            "vault config is missing: {}\n",
            root.join("vault.toml").display()
        )
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
