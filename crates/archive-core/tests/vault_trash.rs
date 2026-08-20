use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, CollectionDefinition, ItemLinkDefinition, Vault};

#[test]
fn move_and_restore_preserve_content_references_and_avoid_collisions() {
    let root = temp_path("vault-trash");
    let source = root.with_extension("png");
    fs::write(&source, b"preserved bytes").unwrap();
    let vault = Vault::create(&root).unwrap();
    let item = vault.add_artwork_item(AddArtworkItem {
        source_file: source.clone(), home_subvault: "Paintings".into(), title: "Blue".into(),
        creator: Some("Artist".into()), year: Some("2020".into()), saving_reason: None,
    }).unwrap();
    let collection = vault.create_collection(CollectionDefinition { name: "Favorites".into(), purpose: "Keep".into(), description: None }).unwrap();
    vault.add_item_to_collection(collection.id(), item.id()).unwrap();
    let source_item = vault.add_artwork_item(AddArtworkItem { source_file: source.clone(), home_subvault: "Paintings".into(), title: "Source".into(), creator: Some("Artist".into()), year: Some("2021".into()), saving_reason: None }).unwrap();
    vault.add_item_link(source_item.id(), ItemLinkDefinition { link_type: "related".into(), target: item.id().into(), label: "Inspired by".into() }).unwrap();
    let original = item.item_folder().to_path_buf();
    let trashed = vault.move_item_to_trash(item.id()).unwrap();
    assert!(trashed.item_folder().starts_with(root.join("trash/Paintings")));
    assert_eq!(fs::read(trashed.item_folder().join("files").join(source.file_name().unwrap())).unwrap(), b"preserved bytes");
    assert!(!vault.browse_artwork_items("Paintings").unwrap().iter().any(|entry| entry.saved_item().id() == item.id()));
    assert!(!vault.review_queue().unwrap().iter().any(|entry| entry.saved_item().id() == item.id()));
    vault.rebuild_metadata_index().unwrap();
    assert!(vault.search_metadata("Blue").unwrap().is_empty());
    let trash = vault.list_trashed_items().unwrap();
    assert_eq!(trash[0].id(), item.id());
    assert_eq!(trash[0].collections(), vec!["Favorites"]);
    assert_eq!(trash[0].incoming_item_links()[0].source_item_id(), source_item.id());
    assert!(vault.item_details(source_item.id()).unwrap().item_links()[0].target_in_vault_trash());
    let trashed_record = trashed.item_folder().join("record.md");
    let valid_record = fs::read(&trashed_record).unwrap();
    fs::write(&trashed_record, b"---\nid: [broken\n---\n").unwrap();
    assert!(vault.vault_problems().unwrap().iter().any(|problem| problem.path() == trashed_record));
    fs::write(&trashed_record, valid_record).unwrap();

    fs::create_dir_all(&original).unwrap();
    let restored = vault.restore_trashed_item(item.id()).unwrap();
    assert_eq!(restored.id(), item.id());
    assert_ne!(restored.item_folder(), original);
    assert!(restored.item_folder().file_name().unwrap().to_string_lossy().ends_with(" (2)"));
    drop(vault);
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(reopened.item_details(item.id()).unwrap().collections(), vec!["Favorites"]);
    assert!(!reopened.item_details(source_item.id()).unwrap().item_links()[0].target_in_vault_trash());
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(source).unwrap();
}

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gg-{label}-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()))
}
