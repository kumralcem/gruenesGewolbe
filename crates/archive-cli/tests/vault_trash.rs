use gruenes_gewolbe_core::{AddArtworkItem, CollectionDefinition, ItemLinkDefinition, Vault};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn cli_moves_and_restores_an_explicit_item() {
    let root = temp("cli-trash");
    let source = root.with_extension("png");
    fs::write(&source, b"x").unwrap();
    let vault = Vault::create(&root).unwrap();
    let item = vault
        .add_artwork_item(AddArtworkItem {
            source_file: source.clone(),
            home_subvault: "Paintings".into(),
            title: "T".into(),
            creator: Some("C".into()),
            year: Some("2020".into()),
            saving_reason: None,
        })
        .unwrap();
    let exe = env!("CARGO_BIN_EXE_ggvault");
    let moved = Command::new(exe)
        .args(["trash-item", root.to_str().unwrap(), item.id()])
        .output()
        .unwrap();
    assert!(moved.status.success());
    assert!(String::from_utf8(moved.stdout)
        .unwrap()
        .starts_with("trashed-item\t"));
    let restored = Command::new(exe)
        .args(["restore-item", root.to_str().unwrap(), item.id()])
        .output()
        .unwrap();
    assert!(restored.status.success());
    assert!(String::from_utf8(restored.stdout)
        .unwrap()
        .starts_with("restored-item\t"));
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(source).unwrap();
}

#[test]
fn cli_permanent_delete_requires_confirmation_and_reports_impact() {
    let root = temp("cli-permanent-delete");
    let source = root.with_extension("png");
    fs::write(&source, b"x").unwrap();
    let vault = Vault::create(&root).unwrap();
    let target = vault
        .add_artwork_item(AddArtworkItem {
            source_file: source.clone(),
            home_subvault: "Paintings".into(),
            title: "Target".into(),
            creator: None,
            year: None,
            saving_reason: None,
        })
        .unwrap();
    let source_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file: source.clone(),
            home_subvault: "Paintings".into(),
            title: "Source".into(),
            creator: None,
            year: None,
            saving_reason: None,
        })
        .unwrap();
    let collection = vault
        .create_collection(CollectionDefinition {
            name: "Favorites".into(),
            purpose: "Keep".into(),
            description: None,
        })
        .unwrap();
    vault
        .add_item_to_collection(collection.id(), target.id())
        .unwrap();
    vault
        .add_item_link(
            source_item.id(),
            ItemLinkDefinition {
                link_type: "references".into(),
                target: target.id().into(),
                label: "Target link".into(),
            },
        )
        .unwrap();
    vault.move_item_to_trash(target.id()).unwrap();
    let exe = env!("CARGO_BIN_EXE_ggvault");
    let refused = Command::new(exe)
        .args([
            "permanently-delete-item",
            root.to_str().unwrap(),
            target.id(),
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(vault.list_trashed_items().unwrap().len(), 1);
    let deleted = Command::new(exe)
        .args([
            "permanently-delete-item",
            root.to_str().unwrap(),
            target.id(),
            "--confirm",
            target.id(),
        ])
        .output()
        .unwrap();
    assert!(deleted.status.success());
    let output = String::from_utf8(deleted.stdout).unwrap();
    assert!(output.contains(&format!("permanently-deleted-item\t{}", target.id())));
    assert!(output.contains("affected-collection\tFavorites"));
    assert!(output.contains(&format!(
        "removed-incoming-item-link\t{}\tTarget link",
        source_item.id()
    )));
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(source).unwrap();
}
fn temp(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "gg-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
