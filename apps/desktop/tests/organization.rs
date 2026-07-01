use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{CollectionDefinition, ItemLinkDefinition, TagDefinition};
use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_supports_tags_collections_and_item_links() {
    let root = temp_path("desktop-organization-vault");
    let source_dir = temp_path("desktop-organization-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let imported = shell
        .import_paintings_folder(&source_dir)
        .expect("import paintings");
    let item_id = imported[0].id().to_string();

    shell
        .upsert_tag(TagDefinition {
            name: "night palette".to_string(),
            aliases: vec!["nocturne colors".to_string()],
            meaning: None,
        })
        .expect("upsert tag");
    shell
        .add_tags_to_item(&item_id, vec!["nocturne colors".to_string()])
        .expect("add normalized tag");

    let collection = shell
        .create_collection(CollectionDefinition {
            name: "Night References".to_string(),
            purpose: "Visual research".to_string(),
            description: None,
        })
        .expect("create collection");
    shell
        .add_item_to_collection(collection.id(), &item_id)
        .expect("add item to collection");

    shell
        .add_item_link(
            &item_id,
            ItemLinkDefinition {
                link_type: "url".to_string(),
                target: "https://example.com/night-palette-notes".to_string(),
                label: "Night palette notes".to_string(),
            },
        )
        .expect("add item link");

    let details = shell.item_details(&item_id).expect("read details");
    assert_eq!(details.tags(), vec!["night palette"]);
    assert_eq!(details.collections(), vec!["Night References"]);
    assert_eq!(details.item_links()[0].label(), "Night palette notes");

    let results = shell
        .search_metadata("night palette notes")
        .expect("search item link");
    assert_eq!(results[0].saved_item().id(), item_id);

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
