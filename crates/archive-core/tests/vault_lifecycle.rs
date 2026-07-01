use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::Vault;

#[test]
fn user_can_create_a_vault_as_ordinary_files_and_reopen_it() {
    let root = temp_path("create-reopen");

    let created = Vault::create(&root).expect("create vault");
    assert_eq!(created.root(), root.as_path());

    assert!(root.join("vault.toml").is_file());
    assert!(root.join("subvaults").is_dir());
    assert!(root.join("collections").is_dir());

    let visible_config = fs::read_to_string(root.join("vault.toml")).expect("read vault config");
    assert!(visible_config.contains("format_version = 1"));
    assert!(visible_config.contains("name = \"Personal Archive\""));

    let reopened = Vault::open(&root).expect("reopen vault");
    assert_eq!(reopened.root(), root.as_path());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn user_can_open_a_copied_vault_without_hidden_app_state() {
    let original = temp_path("original");
    let copied = temp_path("copied");

    Vault::create(&original).expect("create original vault");
    copy_dir(original.join("subvaults"), copied.join("subvaults"));
    copy_dir(original.join("collections"), copied.join("collections"));
    fs::create_dir_all(&copied).expect("create copied root");
    fs::copy(original.join("vault.toml"), copied.join("vault.toml")).expect("copy config");

    assert!(!copied.join(".gruenesgewolbe").exists());

    let opened = Vault::open(&copied).expect("open copied vault");
    assert_eq!(opened.root(), copied.as_path());

    fs::remove_dir_all(&original).expect("clean original vault");
    fs::remove_dir_all(&copied).expect("clean copied vault");
}

#[test]
fn validation_reports_clear_errors_for_invalid_vault_folders() {
    let root = temp_path("invalid");
    fs::create_dir_all(&root).expect("create invalid root");

    let error = Vault::validate(&root).expect_err("invalid vault should fail validation");
    assert_eq!(
        error.to_string(),
        format!(
            "vault config is missing: {}",
            root.join("vault.toml").display()
        )
    );

    fs::remove_dir_all(&root).expect("clean invalid root");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}

fn copy_dir(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::create_dir_all(to).expect("create destination directory");

    for entry in fs::read_dir(from).expect("read source directory") {
        let entry = entry.expect("read directory entry");
        let destination = to.join(entry.file_name());
        if entry.file_type().expect("read file type").is_dir() {
            copy_dir(entry.path(), destination);
        } else {
            fs::copy(entry.path(), destination).expect("copy file");
        }
    }
}
