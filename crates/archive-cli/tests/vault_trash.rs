use std::{fs, path::PathBuf, process::Command, time::{SystemTime, UNIX_EPOCH}};
use gruenes_gewolbe_core::{AddArtworkItem, Vault};

#[test]
fn cli_moves_and_restores_an_explicit_item() {
    let root = temp("cli-trash"); let source = root.with_extension("png"); fs::write(&source, b"x").unwrap();
    let vault = Vault::create(&root).unwrap();
    let item = vault.add_artwork_item(AddArtworkItem { source_file: source.clone(), home_subvault: "Paintings".into(), title: "T".into(), creator: Some("C".into()), year: Some("2020".into()), saving_reason: None }).unwrap();
    let exe = env!("CARGO_BIN_EXE_ggvault");
    let moved = Command::new(exe).args(["trash-item", root.to_str().unwrap(), item.id()]).output().unwrap();
    assert!(moved.status.success()); assert!(String::from_utf8(moved.stdout).unwrap().starts_with("trashed-item\t"));
    let restored = Command::new(exe).args(["restore-item", root.to_str().unwrap(), item.id()]).output().unwrap();
    assert!(restored.status.success()); assert!(String::from_utf8(restored.stdout).unwrap().starts_with("restored-item\t"));
    fs::remove_dir_all(root).unwrap(); fs::remove_file(source).unwrap();
}
fn temp(label: &str) -> PathBuf { std::env::temp_dir().join(format!("gg-{label}-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())) }
