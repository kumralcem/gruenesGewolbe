use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    CopiedImage, ExtractedTextCapture, ManualFallbackCapture, SourceCaptureResult,
    SourceExtraction, SourceExtractionRequest, SourceExtractor, SourceLinkCapture, Vault,
};

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

#[test]
fn url_capture_returns_a_manual_fallback_prompt_when_extraction_is_blocked() {
    let root = temp_path("capture-url-fallback-vault");
    let vault = Vault::create(&root).expect("create vault");
    let extractor = FakeExtractor::new(SourceExtraction::NeedsManualFallback {
        reason: "source requires clipboard capture".to_string(),
    });

    let result = vault
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://x.com/example/status/123".to_string(),
                title: "Blocked source".to_string(),
                saving_reason: Some("Useful discussion seed".to_string()),
            },
            &extractor,
        )
        .expect("request source capture");

    let SourceCaptureResult::NeedsManualFallback(prompt) = result else {
        panic!("expected manual fallback prompt");
    };

    assert_eq!(prompt.source_link(), "https://x.com/example/status/123");
    assert_eq!(prompt.title(), "Blocked source");
    assert_eq!(prompt.saving_reason(), Some("Useful discussion seed"));
    assert_eq!(prompt.reason(), "source requires clipboard capture");
    assert_eq!(
        extractor.last_request().source_link,
        "https://x.com/example/status/123"
    );
    assert!(vault
        .browse_idea_sources()
        .expect("browse idea sources")
        .is_empty());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn url_capture_persists_extracted_cleaned_text_when_extraction_succeeds() {
    let root = temp_path("capture-url-extracted-vault");
    let vault = Vault::create(&root).expect("create vault");
    let extractor = FakeExtractor::new(SourceExtraction::ExtractedText {
        title: Some("Extracted essay title".to_string()),
        cleaned_text: "Full main content from the extracted source.".to_string(),
    });

    let result = vault
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://example.com/essay".to_string(),
                title: "Pasted fallback title".to_string(),
                saving_reason: Some("Quote source for archive design".to_string()),
            },
            &extractor,
        )
        .expect("capture extracted source");

    let SourceCaptureResult::Captured(captured) = result else {
        panic!("expected captured source");
    };

    assert_eq!(captured.home_subvault(), "Idea Sources");
    assert_eq!(
        extractor.last_request().source_link,
        "https://example.com/essay"
    );
    assert_eq!(
        fs::read_to_string(
            captured
                .item_folder()
                .join("source-copies")
                .join("cleaned-text.md")
        )
        .expect("read cleaned text"),
        "Full main content from the extracted source.\n"
    );

    let record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read item record");
    assert!(record.contains("title: Extracted essay title"));
    assert!(record.contains("source_link: https://example.com/essay"));
    assert!(record.contains("source_copy: source-copies/cleaned-text.md"));
    assert!(record.contains("capture_method: extracted-text"));
    assert!(record.contains("## Saving Reason\n\nQuote source for archive design\n"));

    let results = vault
        .search_metadata("archive design")
        .expect("search extracted source");
    assert_eq!(results[0].saved_item().id(), captured.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn url_capture_preserves_extracted_visual_bytes_with_source_provenance() {
    let root = temp_path("capture-url-image-vault");
    let vault = Vault::create(&root).expect("create vault");
    let extractor = FakeExtractor::new(SourceExtraction::ExtractedImage {
        title: Some("The Great Wave".to_string()),
        file_name: "great-wave.jpg".to_string(),
        bytes: b"best available image bytes".to_vec(),
    });

    let result = vault
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://commons.wikimedia.org/wiki/File:The_Great_Wave.jpg".into(),
                title: "Wikimedia capture".into(),
                saving_reason: Some("Composition reference".into()),
            },
            &extractor,
        )
        .expect("capture extracted image");

    let SourceCaptureResult::Captured(captured) = result else {
        panic!("expected captured visual source");
    };
    assert_eq!(captured.home_subvault(), "Paintings");
    assert_eq!(
        fs::read(captured.item_folder().join("files/great-wave.jpg"))
            .expect("read preserved image"),
        b"best available image bytes"
    );
    let record = fs::read_to_string(captured.item_folder().join("record.md"))
        .expect("read Item Record");
    assert!(record.contains("item_type: artwork"));
    assert!(record.contains("title: The Great Wave"));
    assert!(record.contains("source_link: https://commons.wikimedia.org/wiki/File:The_Great_Wave.jpg"));
    assert!(record.contains("primary_file: files/great-wave.jpg"));
    assert!(record.contains("capture_method: extracted-image"));
    assert!(record.contains("imported_at: "));
    assert!(!record.contains("capture-staging"));
    assert!(!record.contains("import_original_filename:"));
    assert!(!record.contains("import_source_path:"));
    assert!(!record.contains("import_source_folder:"));
    let staging = root.join(".gruenesgewolbe").join("capture-staging");
    assert!(
        !staging.exists()
            || fs::read_dir(&staging)
                .expect("read capture staging")
                .next()
                .is_none()
    );
    assert_eq!(
        vault.item_details(captured.id()).expect("read captured details").source_link(),
        Some("https://commons.wikimedia.org/wiki/File:The_Great_Wave.jpg")
    );
    assert_eq!(
        vault.browse_artwork_items("Paintings").expect("browse captured artwork")[0]
            .saved_item()
            .id(),
        captured.id()
    );

    fs::remove_dir_all(root).expect("clean temp vault");
}

#[derive(Debug)]
struct FakeExtractor {
    response: SourceExtraction,
    requests: std::cell::RefCell<Vec<SourceExtractionRequest>>,
}

impl FakeExtractor {
    fn new(response: SourceExtraction) -> Self {
        Self {
            response,
            requests: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn last_request(&self) -> SourceExtractionRequest {
        self.requests
            .borrow()
            .last()
            .expect("source extraction request")
            .clone()
    }
}

impl SourceExtractor for FakeExtractor {
    fn extract(&self, request: SourceExtractionRequest) -> SourceExtraction {
        self.requests.borrow_mut().push(request);
        self.response.clone()
    }
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
