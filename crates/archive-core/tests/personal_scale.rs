use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::Vault;

const PERSONAL_SCALE_ITEM_COUNT: usize = 5_000;

#[test]
fn generated_personal_scale_vault_opens_browses_searches_and_rebuilds() {
    let root = temp_path("personal-scale-vault");
    Vault::create(&root).expect("create generated Vault");
    generate_artwork_records(&root, PERSONAL_SCALE_ITEM_COUNT);
    for index in 0..10 {
        fs::remove_file(
            root.join(".gruenesgewolbe/thumbnails")
                .join(format!("generated-item-{index:05}.png")),
        )
        .expect("remove a generated cached Thumbnail Preview");
    }

    let open_started = Instant::now();
    let reopened = Vault::open(&root).expect("open generated Vault");
    let open_elapsed = open_started.elapsed();

    let browse_started = Instant::now();
    let artwork = reopened
        .browse_artwork_items("Paintings")
        .expect("browse generated Saved Items");
    let browse_elapsed = browse_started.elapsed();
    assert_eq!(artwork.len(), PERSONAL_SCALE_ITEM_COUNT);
    assert_eq!(artwork[0].title(), "Generated Artwork 04999");
    for index in 0..10 {
        assert!(
            root.join(".gruenesgewolbe/thumbnails")
                .join(format!("generated-item-{index:05}.png"))
                .is_file(),
            "browse should rebuild the uncached Thumbnail Preview"
        );
    }

    let rebuild_started = Instant::now();
    let rebuilt = reopened
        .rebuild_metadata_index()
        .expect("rebuild generated Derived Index");
    let rebuild_elapsed = rebuild_started.elapsed();
    assert_eq!(rebuilt.indexed_items(), PERSONAL_SCALE_ITEM_COUNT);
    assert!(rebuilt.omitted_paths().is_empty());

    let search_started = Instant::now();
    let results = reopened
        .search_metadata("personal-scale-marker-04999")
        .expect("search generated Vault");
    let search_elapsed = search_started.elapsed();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title(), "Generated Artwork 04999");

    report_timings(
        open_elapsed,
        browse_elapsed,
        rebuild_elapsed,
        search_elapsed,
    );
    fs::remove_dir_all(root).expect("clean generated Vault");
}

fn generate_artwork_records(root: &std::path::Path, count: usize) {
    let items_root = root.join("subvaults/Paintings/items");
    let thumbnails_root = root.join(".gruenesgewolbe/thumbnails");
    fs::create_dir_all(&thumbnails_root).expect("create generated thumbnail cache");
    let shared_image = root.join("generated-image.png");
    image::RgbImage::from_pixel(1, 1, image::Rgb([20, 40, 60]))
        .save(&shared_image)
        .expect("write shared generated image");
    for index in 0..count {
        let item_id = format!("generated-item-{index:05}");
        let item_folder = items_root.join(&item_id);
        let files_folder = item_folder.join("files");
        fs::create_dir_all(&files_folder).expect("create generated Item Folder");
        fs::hard_link(&shared_image, files_folder.join("generated.png"))
            .expect("link generated Preserved File");
        fs::hard_link(
            &shared_image,
            thumbnails_root.join(format!("{item_id}.png")),
        )
        .expect("link generated Thumbnail Preview");
        fs::write(
            item_folder.join("record.md"),
            format!(
                "---\n\
id: {item_id}\n\
item_type: artwork\n\
home_subvault: Paintings\n\
title: Generated Artwork {index:05}\n\
creator: Generated Artist {artist:03}\n\
year: '{year}'\n\
primary_file: files/generated.png\n\
import_original_filename: generated-{index:05}.png\n\
import_source_path: /generated/generated-{index:05}.png\n\
import_source_folder: /generated\n\
imported_at: '{index:05}'\n\
file_fingerprint: generated-{index:05}\n\
duplicate_candidates: []\n\
review_reasons: []\n\
review_status: reviewed\n\
---\n\n\
# Generated Artwork {index:05}\n\n\
## Saving Reason\n\n\
personal-scale-marker-{index:05}\n",
                artist = index % 200,
                year = 1800 + index % 225,
            ),
        )
        .expect("write generated Item Record");
    }
    fs::remove_file(shared_image).expect("remove shared image staging link");
}

fn report_timings(open: Duration, browse: Duration, rebuild: Duration, search: Duration) {
    eprintln!(
        "personal-scale timings for {PERSONAL_SCALE_ITEM_COUNT} Saved Items: open={open:?}, browse={browse:?}, rebuild={rebuild:?}, search={search:?}"
    );
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
