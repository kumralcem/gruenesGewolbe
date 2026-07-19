use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    AddArtworkItem, ManualFallbackCapture, SourceCaptureResult, SourceExtraction,
    SourceExtractionRequest, SourceExtractor, SourceLinkCapture, Vault,
};

#[test]
fn paintings_import_skips_byte_identical_files_instead_of_creating_review_work() {
    let root = temp_path("duplicate-file-vault");
    let existing_source_dir = temp_path("duplicate-file-existing");
    let import_source_dir = temp_path("duplicate-file-import");
    fs::create_dir_all(&existing_source_dir).expect("create existing source directory");
    fs::create_dir_all(&import_source_dir).expect("create import source directory");
    let existing_source = existing_source_dir.join("existing.jpg");
    let duplicate_source = import_source_dir.join("Duplicate Artist - 2025 - Duplicate Study.jpg");
    fs::write(&existing_source, b"same image bytes").expect("write existing image");
    fs::write(&duplicate_source, b"same image bytes").expect("write duplicate image");

    let vault = Vault::create(&root).expect("create vault");
    let existing = vault
        .add_artwork_item(AddArtworkItem {
            source_file: existing_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save existing artwork");

    let imported = vault
        .import_paintings_folder(&import_source_dir)
        .expect("import duplicate candidate");
    assert!(imported.is_empty());
    assert!(vault.item_details(existing.id()).is_ok());
    assert_eq!(
        vault
            .browse_artwork_items("Paintings")
            .expect("browse paintings")
            .len(),
        1
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&existing_source_dir).expect("clean existing source directory");
    fs::remove_dir_all(&import_source_dir).expect("clean import source directory");
}

#[test]
fn manual_capture_warns_about_exact_source_link_duplicate_candidates_without_blocking_save() {
    let root = temp_path("duplicate-source-link-vault");
    let vault = Vault::create(&root).expect("create vault");

    let first = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/same-source".to_string(),
            title: "First source capture".to_string(),
            saving_reason: Some("First reason".to_string()),
            copied_text: Some("First copied text".to_string()),
            copied_image: None,
        })
        .expect("capture first source");

    let second = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/same-source".to_string(),
            title: "Second source capture".to_string(),
            saving_reason: Some("Second reason".to_string()),
            copied_text: Some("Second copied text".to_string()),
            copied_image: None,
        })
        .expect("capture duplicate source");

    assert_ne!(first.id(), second.id());

    let second_details = vault
        .item_details(second.id())
        .expect("read second details");
    assert_eq!(second_details.review_status(), "needs-review");
    assert_eq!(second_details.duplicate_candidates().len(), 1);
    assert_eq!(
        second_details.duplicate_candidates()[0].item_id(),
        first.id()
    );
    assert_eq!(
        second_details.duplicate_candidates()[0].signal(),
        "source-link"
    );

    let record = fs::read_to_string(second.item_folder().join("record.md")).expect("read record");
    assert!(record.contains(&format!(
        "duplicate_candidates: {} | source-link",
        first.id()
    )));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn successful_url_capture_warns_about_exact_source_link_duplicate_candidates_without_blocking_save()
{
    let root = temp_path("duplicate-url-source-link-vault");
    let vault = Vault::create(&root).expect("create vault");

    let first = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/same-source".to_string(),
            title: "First source capture".to_string(),
            saving_reason: Some("First reason".to_string()),
            copied_text: Some("First copied text".to_string()),
            copied_image: None,
        })
        .expect("capture first source");
    let extractor = FakeExtractor::new(SourceExtraction::ExtractedText {
        title: Some("Extracted duplicate source".to_string()),
        cleaned_text: "Full extracted text for the duplicate source.".to_string(),
    });

    let result = vault
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://example.com/same-source".to_string(),
                title: "Pasted fallback title".to_string(),
                saving_reason: Some("Second reason".to_string()),
            },
            &extractor,
        )
        .expect("capture duplicate source link");

    let SourceCaptureResult::Captured(second) = result else {
        panic!("expected captured source");
    };
    assert_ne!(first.id(), second.id());

    let second_details = vault
        .item_details(second.id())
        .expect("read second details");
    assert_eq!(second_details.review_status(), "needs-review");
    assert_eq!(second_details.duplicate_candidates().len(), 1);
    assert_eq!(
        second_details.duplicate_candidates()[0].item_id(),
        first.id()
    );
    assert_eq!(
        second_details.duplicate_candidates()[0].signal(),
        "source-link"
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn repeated_paintings_import_does_not_create_exact_duplicate_review_work() {
    let root = temp_path("duplicate-provenance-vault");
    let import_source_dir = temp_path("duplicate-provenance-source");
    fs::create_dir_all(&import_source_dir).expect("create import source directory");
    let source = import_source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg");
    fs::write(&source, b"image bytes").expect("write image");

    let vault = Vault::create(&root).expect("create vault");
    let first_import = vault
        .import_paintings_folder(&import_source_dir)
        .expect("first import");
    let second_import = vault
        .import_paintings_folder(&import_source_dir)
        .expect("second import");

    assert!(second_import.is_empty());
    assert_eq!(
        vault
            .browse_artwork_items("Paintings")
            .expect("browse paintings")
            .len(),
        first_import.len()
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&import_source_dir).expect("clean import source directory");
}

#[test]
fn artwork_save_warns_about_descriptive_metadata_duplicate_candidates_without_blocking_save() {
    let root = temp_path("duplicate-metadata-vault");
    let source_dir = temp_path("duplicate-metadata-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let first_source = source_dir.join("first.jpg");
    let second_source = source_dir.join("second.jpg");
    fs::write(&first_source, b"first image bytes").expect("write first image");
    fs::write(&second_source, b"second image bytes").expect("write second image");

    let vault = Vault::create(&root).expect("create vault");
    let first = vault
        .add_artwork_item(AddArtworkItem {
            source_file: first_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save first artwork");
    let second = vault
        .add_artwork_item(AddArtworkItem {
            source_file: second_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: None,
        })
        .expect("save second artwork");

    let duplicate_details = vault
        .item_details(second.id())
        .expect("read duplicate details");
    assert_eq!(duplicate_details.review_status(), "needs-review");
    assert!(duplicate_details
        .duplicate_candidates()
        .iter()
        .any(|candidate| candidate.item_id() == first.id()
            && candidate.signal() == "descriptive-metadata"));

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

#[derive(Debug)]
struct FakeExtractor {
    response: SourceExtraction,
}

impl FakeExtractor {
    fn new(response: SourceExtraction) -> Self {
        Self { response }
    }
}

impl SourceExtractor for FakeExtractor {
    fn extract(&self, _request: SourceExtractionRequest) -> SourceExtraction {
        self.response.clone()
    }
}
