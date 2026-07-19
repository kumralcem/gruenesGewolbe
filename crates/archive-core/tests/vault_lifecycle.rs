use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{Vault, VaultOpen};

#[test]
fn user_can_create_a_vault_as_ordinary_files_and_reopen_it() {
    let root = temp_path("create-reopen");

    let created = Vault::create(&root).expect("create vault");
    assert_eq!(created.root(), root.as_path());

    assert!(root.join("vault.toml").is_file());
    assert!(root.join("subvaults").is_dir());
    assert!(root.join("collections").is_dir());

    let visible_config = fs::read_to_string(root.join("vault.toml")).expect("read vault config");
    assert!(visible_config.contains("format_version = 2"));
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
fn vault_creation_refuses_an_unrelated_non_empty_folder() {
    let root = temp_path("non-empty");
    fs::create_dir_all(&root).expect("create non-empty root");
    fs::write(root.join("unrelated.txt"), "keep me").expect("write unrelated file");

    let error = Vault::create(&root).expect_err("non-empty folder should be refused");

    assert_eq!(
        error.to_string(),
        format!("vault folder is not empty: {}", root.display())
    );
    assert_eq!(
        fs::read_to_string(root.join("unrelated.txt")).expect("read unrelated file"),
        "keep me"
    );
    assert!(!root.join("vault.toml").exists());
    assert!(!root.join("subvaults").exists());
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean non-empty root");
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

#[test]
fn opening_a_recognizable_incomplete_vault_proposes_missing_structure_without_changing_it() {
    let root = temp_path("repair-proposal");
    fs::create_dir_all(&root).expect("create vault root");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write valid config");

    let outcome = Vault::open_or_repair(&root).expect("inspect incomplete vault");
    let VaultOpen::RepairRequired(proposal) = outcome else {
        panic!("missing structure should require repair");
    };

    assert_eq!(proposal.root(), root.as_path());
    assert_eq!(
        proposal.directories(),
        &[root.join("subvaults"), root.join("collections")]
    );
    assert!(!root.join("subvaults").exists());
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean incomplete vault");
}

#[test]
fn confirming_repair_creates_only_missing_structure_and_opens_the_vault() {
    let root = temp_path("confirmed-repair");
    let record_path = root
        .join("subvaults")
        .join("Paintings")
        .join("items")
        .join("Damaged Record")
        .join("record.md");
    fs::create_dir_all(record_path.parent().expect("record parent"))
        .expect("create existing item structure");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write valid config");
    fs::write(&record_path, b"not valid frontmatter\nkeep these bytes\n")
        .expect("write malformed item record");

    let VaultOpen::RepairRequired(proposal) =
        Vault::open_or_repair(&root).expect("inspect incomplete vault")
    else {
        panic!("missing collections should require repair");
    };
    assert_eq!(proposal.directories(), &[root.join("collections")]);

    let repaired = proposal.confirm().expect("confirm safe repair");

    assert_eq!(repaired.root(), root.as_path());
    assert!(root.join("collections").is_dir());
    assert!(!root.join(".gruenesgewolbe").exists());
    assert_eq!(
        fs::read(&record_path).expect("read malformed record after repair"),
        b"not valid frontmatter\nkeep these bytes\n"
    );

    fs::remove_dir_all(&root).expect("clean repaired vault");
}

#[test]
fn opening_rejects_structural_conflicts_instead_of_proposing_repair() {
    let root = temp_path("unsafe-repair");
    fs::create_dir_all(&root).expect("create vault root");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write valid config");
    fs::write(root.join("subvaults"), b"canonical content")
        .expect("write conflicting canonical file");

    let error = Vault::open_or_repair(&root).expect_err("structural conflict should fail");

    assert_eq!(
        error.to_string(),
        format!(
            "required vault directory conflicts with an existing file: {}",
            root.join("subvaults").display()
        )
    );
    assert_eq!(
        fs::read(root.join("subvaults")).expect("read conflicting file"),
        b"canonical content"
    );
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean conflicting vault");
}

#[test]
fn opening_rejects_invalid_configuration_without_proposing_or_creating_structure() {
    let root = temp_path("invalid-repair");
    fs::create_dir_all(&root).expect("create vault root");
    fs::write(root.join("vault.toml"), "name = \"Unversioned\"\n").expect("write invalid config");

    let error = Vault::open_or_repair(&root).expect_err("invalid config should fail");

    assert_eq!(
        error.to_string(),
        format!(
            "vault config has an unsupported format: {}",
            root.join("vault.toml").display()
        )
    );
    assert!(!root.join("subvaults").exists());
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean invalid vault");
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
