use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::ManualFallbackCapture;
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

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
