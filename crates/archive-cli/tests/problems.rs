use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, Vault};

#[test]
fn cli_lists_vault_problems_as_structured_output() {
    let root = temp_path("cli-vault-problems-vault");
    let source = temp_path("cli-vault-problems-source").with_extension("png");
    fs::write(&source, b"source image fixture").expect("write source image");
    let vault = Vault::create(&root).expect("create Vault");
    let item = vault
        .add_artwork_item(AddArtworkItem {
            source_file: source.clone(),
            home_subvault: "Paintings".to_string(),
            creator: Some("Test Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Broken Record".to_string(),
            saving_reason: None,
        })
        .expect("save artwork");
    let record_path = item.item_folder().join("record.md");
    fs::write(&record_path, "---\nid: [broken\n---\n").expect("malform Item Record");

    let output = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("problems")
        .arg(&root)
        .output()
        .expect("run problems command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("problems stdout");
    let fields = stdout.trim().split('\t').collect::<Vec<_>>();
    assert_eq!(fields[0], "vault-problem");
    assert_eq!(fields[1], record_path.display().to_string());
    assert!(fields[2].contains("YAML parse error"));

    fs::remove_dir_all(&root).expect("clean Vault");
    fs::remove_file(&source).expect("clean source image");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
