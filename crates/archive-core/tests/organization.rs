use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    AddArtworkItem, CollectionDefinition, ItemLinkDefinition, TagDefinition, Vault,
};

#[test]
fn user_can_add_tags_normalized_through_the_vault_tag_registry() {
    let root = temp_path("organization-tags-vault");
    let source_dir = temp_path("organization-tags-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"nocturne bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork item");

    vault
        .upsert_tag(TagDefinition {
            name: "night palette".to_string(),
            aliases: vec!["nocturne colors".to_string()],
            meaning: Some("Dark color references for night scenes".to_string()),
        })
        .expect("create tag registry entry");

    let details = vault
        .add_tags_to_item(saved_item.id(), vec!["Nocturne Colors".to_string()])
        .expect("add normalized tag");

    assert_eq!(details.tags(), vec!["night palette"]);
    assert_eq!(
        fs::read_to_string(root.join("tag-registry.md")).expect("read tag registry"),
        "# Tag Registry\n\n## night palette\n\nAliases: nocturne colors\n\nDark color references for night scenes\n"
    );

    assert!(!details.tags().contains(&"nocturne colors"));

    let results = vault
        .search_metadata("night palette")
        .expect("search canonical tag");
    assert_eq!(results[0].saved_item().id(), saved_item.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn user_can_create_a_collection_and_add_items_without_moving_item_folders() {
    let root = temp_path("organization-collection-vault");
    let source_dir = temp_path("organization-collection-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"nocturne bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork item");
    let original_folder = saved_item.item_folder().to_path_buf();

    let collection = vault
        .create_collection(CollectionDefinition {
            name: "Night References".to_string(),
            purpose: "Visual research".to_string(),
            description: Some("Paintings to revisit for night palette work".to_string()),
        })
        .expect("create collection");
    vault
        .add_item_to_collection(collection.id(), saved_item.id())
        .expect("add item to collection");

    assert_eq!(saved_item.item_folder(), original_folder.as_path());
    assert!(original_folder.is_dir());

    let collection_file = root.join("collections").join("night-references.md");
    let collection_record = fs::read_to_string(&collection_file).expect("read collection file");
    assert!(collection_record.contains("name: Night References"));
    assert!(collection_record.contains("purpose: Visual research"));
    assert!(collection_record.contains("## Items\n\n- "));
    assert!(collection_record.contains(saved_item.id()));

    let details = vault.item_details(saved_item.id()).expect("read details");
    assert_eq!(details.collections(), vec!["Night References"]);

    let results = vault
        .search_metadata("night references")
        .expect("search collection");
    assert_eq!(results[0].saved_item().id(), saved_item.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn item_details_reads_collection_membership_from_collection_files() {
    let root = temp_path("organization-collection-file-vault");
    let source_dir = temp_path("organization-collection-file-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"nocturne bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork item");
    let collection = vault
        .create_collection(CollectionDefinition {
            name: "Night References".to_string(),
            purpose: "Visual research".to_string(),
            description: Some("Paintings to revisit for night palette work".to_string()),
        })
        .expect("create collection");
    vault
        .add_item_to_collection(collection.id(), saved_item.id())
        .expect("add item to collection");

    let record_path = saved_item.item_folder().join("record.md");
    let record = fs::read_to_string(&record_path).expect("read item record");
    fs::write(
        &record_path,
        record.replace("collections: Night References\n", ""),
    )
    .expect("remove item backreference");

    let details = vault.item_details(saved_item.id()).expect("read details");
    assert_eq!(details.collections(), vec!["Night References"]);

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn user_can_add_item_links_without_moving_the_home_subvault() {
    let root = temp_path("organization-links-vault");
    let source_dir = temp_path("organization-links-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"nocturne bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork item");
    let original_folder = saved_item.item_folder().to_path_buf();

    let details = vault
        .add_item_link(
            saved_item.id(),
            ItemLinkDefinition {
                link_type: "url".to_string(),
                target: "https://example.com/night-palette-notes".to_string(),
                label: "Night palette notes".to_string(),
            },
        )
        .expect("add item link");

    assert_eq!(details.home_subvault(), "Paintings");
    assert_eq!(details.item_folder(), original_folder.as_path());
    assert_eq!(details.item_links().len(), 1);
    assert_eq!(details.item_links()[0].link_type(), "url");
    assert_eq!(
        details.item_links()[0].target(),
        "https://example.com/night-palette-notes"
    );
    assert_eq!(details.item_links()[0].label(), "Night palette notes");

    let record =
        fs::read_to_string(saved_item.item_folder().join("record.md")).expect("read item record");
    assert!(record.contains(
        "## Item Links\n\n- url | Night palette notes | https://example.com/night-palette-notes\n"
    ));

    let results = vault
        .search_metadata("night palette notes")
        .expect("search item link");
    assert_eq!(results[0].saved_item().id(), saved_item.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
