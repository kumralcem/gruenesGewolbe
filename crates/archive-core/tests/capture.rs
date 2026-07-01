use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{CopiedImage, ExtractedTextCapture, ManualFallbackCapture, Vault};

#[test]
fn user_can_capture_a_source_link_with_manual_fallback_text() {
    let root = temp_path("capture-text-vault");
    let vault = Vault::create(&root).expect("create vault");

    let captured = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/nocturne-note".to_string(),
            title: "Nocturne note".to_string(),
            saving_reason: Some("Useful framing for night palettes".to_string()),
            copied_text: Some(
                "Main idea text from the page. This is the part worth keeping.".to_string(),
            ),
            copied_image: None,
        })
        .expect("capture manual fallback text");

    assert_eq!(captured.home_subvault(), "Idea Sources");
    assert!(captured
        .item_folder()
        .join("source-copies")
        .join("cleaned-text.md")
        .is_file());
    assert_eq!(
        fs::read_to_string(
            captured
                .item_folder()
                .join("source-copies")
                .join("cleaned-text.md")
        )
        .expect("read cleaned text"),
        "Main idea text from the page. This is the part worth keeping.\n"
    );

    let record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read item record");
    assert!(record.contains("item_type: idea"));
    assert!(record.contains("home_subvault: Idea Sources"));
    assert!(record.contains("title: Nocturne note"));
    assert!(record.contains("source_link: https://example.com/nocturne-note"));
    assert!(record.contains("source_copy: source-copies/cleaned-text.md"));
    assert!(record.contains("review_status: needs-review"));
    assert!(record.contains("capture_method: manual-fallback"));
    assert!(record.contains("## Saving Reason\n\nUseful framing for night palettes\n"));

    let results = vault
        .search_metadata("night palettes")
        .expect("search captured reason");
    assert_eq!(results[0].saved_item().id(), captured.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn user_can_capture_a_source_link_with_manual_fallback_image_bytes() {
    let root = temp_path("capture-image-vault");
    let vault = Vault::create(&root).expect("create vault");

    let captured = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/nocturne-image".to_string(),
            title: "Copied nocturne image".to_string(),
            saving_reason: Some("Visual reference from blocked source".to_string()),
            copied_text: None,
            copied_image: Some(CopiedImage {
                file_name: "copied-nocturne.png".to_string(),
                bytes: b"copied image bytes".to_vec(),
            }),
        })
        .expect("capture manual fallback image");

    let preserved_file = captured
        .item_folder()
        .join("files")
        .join("copied-nocturne.png");
    assert_eq!(
        fs::read(&preserved_file).expect("read preserved image"),
        b"copied image bytes"
    );

    let record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read item record");
    assert!(record.contains("item_type: idea"));
    assert!(record.contains("source_link: https://example.com/nocturne-image"));
    assert!(record.contains("primary_file: files/copied-nocturne.png"));
    assert!(record.contains("capture_method: manual-fallback"));
    assert!(record.contains("review_status: needs-review"));
    assert!(record.contains("## Saving Reason\n\nVisual reference from blocked source\n"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn user_can_capture_a_source_link_with_extracted_cleaned_text() {
    let root = temp_path("capture-extracted-text-vault");
    let vault = Vault::create(&root).expect("create vault");

    let captured = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/essay".to_string(),
            title: "Essay source".to_string(),
            saving_reason: Some("Quote source for archive design".to_string()),
            cleaned_text: "Full main content without navigation or comments.".to_string(),
        })
        .expect("capture extracted text");

    assert_eq!(
        fs::read_to_string(
            captured
                .item_folder()
                .join("source-copies")
                .join("cleaned-text.md")
        )
        .expect("read cleaned text"),
        "Full main content without navigation or comments.\n"
    );

    let record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read item record");
    assert!(record.contains("capture_method: extracted-text"));
    assert!(record.contains("source_link: https://example.com/essay"));
    assert!(record.contains("source_copy: source-copies/cleaned-text.md"));
    assert!(!record.contains("surrounding_discussion"));

    let results = vault
        .search_metadata("quote source")
        .expect("search extracted capture");
    assert_eq!(results[0].saved_item().id(), captured.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
