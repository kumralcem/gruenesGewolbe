use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, CollectionDefinition, Vault};

#[test]
fn metadata_search_rebuilds_from_visible_item_records_after_derived_state_is_deleted() {
    let root = temp_path("metadata-search-vault");
    let source_dir = temp_path("metadata-search-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let nocturne_source = source_dir.join("nocturne.jpg");
    let garden_source = source_dir.join("garden.png");
    fs::write(&nocturne_source, b"nocturne bytes").expect("write nocturne");
    fs::write(&garden_source, b"garden bytes").expect("write garden");

    let vault = Vault::create(&root).expect("create vault");
    let nocturne = vault
        .add_artwork_item(AddArtworkItem {
            source_file: nocturne_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference for night scenes".to_string()),
        })
        .expect("save nocturne");
    let garden = vault
        .add_artwork_item(AddArtworkItem {
            source_file: garden_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Lee Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Garden Window".to_string(),
            saving_reason: Some("Composition reference".to_string()),
        })
        .expect("save garden");

    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");
    let rebuilt = vault
        .rebuild_metadata_index()
        .expect("rebuild metadata index");
    assert_eq!(rebuilt.indexed_items(), 2);
    assert!(root
        .join(".gruenesgewolbe")
        .join("metadata-index.tsv")
        .is_file());

    let title_results = vault.search_metadata("nocturne").expect("search by title");
    assert_eq!(ids(title_results), vec![nocturne.id().to_string()]);

    let creator_results = vault
        .search_metadata("lee artist")
        .expect("search by creator");
    assert_eq!(ids(creator_results), vec![garden.id().to_string()]);

    let year_results = vault.search_metadata("1884").expect("search by year");
    assert_eq!(ids(year_results), vec![nocturne.id().to_string()]);

    let reason_results = vault
        .search_metadata("composition reference")
        .expect("search by saving reason");
    assert_eq!(ids(reason_results), vec![garden.id().to_string()]);

    let filename_results = vault
        .search_metadata("garden.png")
        .expect("search by filename");
    assert_eq!(ids(filename_results), vec![garden.id().to_string()]);

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn metadata_search_rebuilds_automatically_when_the_derived_index_is_missing() {
    let root = temp_path("metadata-search-auto-rebuild");
    let source_dir = temp_path("metadata-search-auto-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("window-study.webp");
    fs::write(&source_file, b"window bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Lee Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Window Study".to_string(),
            saving_reason: Some("Composition reference".to_string()),
        })
        .expect("save artwork item");

    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");
    let results = vault.search_metadata("window").expect("search metadata");

    assert_eq!(ids(results), vec![saved_item.id().to_string()]);
    assert!(root
        .join(".gruenesgewolbe")
        .join("metadata-index.tsv")
        .is_file());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn metadata_search_refreshes_after_new_artwork_is_saved() {
    let root = temp_path("metadata-search-new-artwork-refresh");
    let source_dir = temp_path("metadata-search-new-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let first_source = source_dir.join("nocturne.jpg");
    let second_source = source_dir.join("garden.png");
    fs::write(&first_source, b"nocturne bytes").expect("write first source image");
    fs::write(&second_source, b"garden bytes").expect("write second source image");

    let vault = Vault::create(&root).expect("create vault");
    vault
        .add_artwork_item(AddArtworkItem {
            source_file: first_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save first artwork item");
    vault
        .search_metadata("nocturne")
        .expect("build initial metadata index");

    let second = vault
        .add_artwork_item(AddArtworkItem {
            source_file: second_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Lee Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Garden Window".to_string(),
            saving_reason: Some("Fresh composition reference".to_string()),
        })
        .expect("save second artwork item");

    let results = vault
        .search_metadata("fresh composition")
        .expect("search after saving second artwork");
    assert_eq!(ids(results), vec![second.id().to_string()]);

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn metadata_search_rebuilds_collection_references_from_collection_files() {
    let root = temp_path("metadata-search-collections-vault");
    let source_dir = temp_path("metadata-search-collections-source");
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
            description: Some("Curated moonlit composition studies".to_string()),
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
    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");

    let results = vault
        .search_metadata("curated moonlit")
        .expect("search collection description");

    assert_eq!(ids(results), vec![saved_item.id().to_string()]);

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

fn ids(results: Vec<gruenes_gewolbe_core::SearchResult>) -> Vec<String> {
    results
        .into_iter()
        .map(|result| result.saved_item().id().to_string())
        .collect()
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
