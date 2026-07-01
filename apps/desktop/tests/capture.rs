use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    ManualFallbackCapture, SourceCaptureResult, SourceExtraction, SourceExtractionRequest,
    SourceExtractor, SourceLinkCapture,
};
use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_captures_manual_fallback_text_into_the_active_vault() {
    let root = temp_path("desktop-capture-vault");
    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");

    let captured = shell
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/nocturne-note".to_string(),
            title: "Nocturne note".to_string(),
            saving_reason: Some("Useful framing for night palettes".to_string()),
            copied_text: Some("Main idea text from the page.".to_string()),
            copied_image: None,
        })
        .expect("capture manual fallback");

    assert_eq!(captured.home_subvault(), "Idea Sources");
    assert!(captured
        .item_folder()
        .join("source-copies")
        .join("cleaned-text.md")
        .is_file());

    let ideas = shell.browse_idea_sources().expect("browse idea sources");
    assert_eq!(ideas.len(), 1);
    assert_eq!(ideas[0].saved_item().id(), captured.id());
    assert_eq!(ideas[0].title(), "Nocturne note");
    assert_eq!(ideas[0].source_link(), "https://example.com/nocturne-note");
    assert_eq!(ideas[0].review_status(), "needs-review");

    let details = shell
        .item_details(captured.id())
        .expect("open idea source details");
    assert_eq!(
        details.source_link(),
        Some("https://example.com/nocturne-note")
    );

    let results = shell
        .search_metadata("night palettes")
        .expect("search captured item");
    assert_eq!(results[0].saved_item().id(), captured.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn desktop_shell_returns_a_manual_fallback_prompt_when_url_extraction_is_blocked() {
    let root = temp_path("desktop-url-fallback-capture-vault");
    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let extractor = FakeExtractor::new(SourceExtraction::NeedsManualFallback {
        reason: "source requires clipboard capture".to_string(),
    });

    let result = shell
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://x.com/example/status/123".to_string(),
                title: "Blocked source".to_string(),
                saving_reason: Some("Useful discussion seed".to_string()),
            },
            &extractor,
        )
        .expect("request URL capture");

    let SourceCaptureResult::NeedsManualFallback(prompt) = result else {
        panic!("expected manual fallback prompt");
    };

    assert_eq!(prompt.source_link(), "https://x.com/example/status/123");
    assert_eq!(prompt.title(), "Blocked source");
    assert_eq!(prompt.saving_reason(), Some("Useful discussion seed"));
    assert_eq!(prompt.reason(), "source requires clipboard capture");
    assert!(shell
        .browse_idea_sources()
        .expect("browse idea sources")
        .is_empty());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn desktop_shell_persists_successful_url_extraction_into_the_active_vault() {
    let root = temp_path("desktop-url-extracted-capture-vault");
    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let extractor = FakeExtractor::new(SourceExtraction::ExtractedText {
        title: Some("Extracted essay title".to_string()),
        cleaned_text: "Full main content from the extracted source.".to_string(),
    });

    let result = shell
        .capture_source_link(
            SourceLinkCapture {
                source_link: "https://example.com/essay".to_string(),
                title: "Pasted fallback title".to_string(),
                saving_reason: Some("Quote source for archive design".to_string()),
            },
            &extractor,
        )
        .expect("capture URL");

    let SourceCaptureResult::Captured(captured) = result else {
        panic!("expected captured source");
    };

    let details = shell.item_details(captured.id()).expect("read details");
    assert_eq!(details.title(), "Extracted essay title");
    assert_eq!(details.source_link(), Some("https://example.com/essay"));
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

    let results = shell
        .search_metadata("archive design")
        .expect("search captured URL");
    assert_eq!(results[0].saved_item().id(), captured.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
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
