use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    AddArtworkItem, AiBudgetMode, AiMetadataSuggestion, AiProvider, AiProviderRequest,
    AiProviderResponse, BetterFileCandidate, ExtractedTextCapture,
};
use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_enriches_captured_ideas_through_active_vault() {
    let root = temp_path("desktop-ai-enrichment-vault");
    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let captured = shell
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/archive-note".to_string(),
            title: "Archive note".to_string(),
            saving_reason: Some("Useful source material".to_string()),
            cleaned_text: "Only cleaned article text.".to_string(),
        })
        .expect("capture idea");

    let provider = FakeProvider::new(AiProviderResponse {
        summary: Some("Short source summary.".to_string()),
        tags: vec!["source material".to_string()],
        suggestions: vec![AiMetadataSuggestion {
            field: "creator".to_string(),
            suggested_value: "Example Author".to_string(),
            confidence: 0.51,
            provenance: "fake-provider:creator".to_string(),
        }],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 2,
    });

    let enrichment = shell
        .enrich_idea_with_ai(captured.id(), AiBudgetMode::Cheap, &provider)
        .expect("enrich idea");

    assert_eq!(enrichment.accepted_summary(), Some("Short source summary."));
    assert_eq!(enrichment.accepted_tags(), vec!["source material"]);
    assert_eq!(enrichment.estimated_cost_cents(), 2);
    assert_eq!(
        provider.last_request().cleaned_text,
        Some("Only cleaned article text.".to_string())
    );

    let details = shell
        .item_details(captured.id())
        .expect("read item details");
    assert_eq!(details.summary(), Some("Short source summary."));
    assert_eq!(details.tags(), vec!["source material"]);
    assert_eq!(details.metadata_suggestions()[0].field(), "creator");

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn desktop_shell_surfaces_better_file_candidates_without_replacing_the_primary_file() {
    let root = temp_path("desktop-better-file-ai-vault");
    let source_dir = temp_path("desktop-better-file-ai-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"desktop small painting bytes").expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let saved = shell
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference".to_string()),
        })
        .expect("save artwork");
    let original_primary_file = shell
        .item_details(saved.id())
        .expect("read details")
        .primary_file()
        .to_path_buf();

    let provider = FakeProvider::new(AiProviderResponse {
        summary: None,
        tags: Vec::new(),
        suggestions: Vec::new(),
        better_file_candidates: vec![BetterFileCandidate {
            source_link: "https://example.com/full-size-nocturne.jpg".to_string(),
            reason: "Higher resolution source image".to_string(),
            provenance: "fake-vision:source-page".to_string(),
        }],
        estimated_cost_cents: 4,
    });

    let enrichment = shell
        .suggest_artwork_metadata_with_ai(saved.id(), AiBudgetMode::Standard, &provider)
        .expect("suggest better file candidate");

    assert_eq!(enrichment.better_file_candidates().len(), 1);

    let details = shell.item_details(saved.id()).expect("read details");
    assert_eq!(details.primary_file(), original_primary_file.as_path());
    assert_eq!(
        details.better_file_candidates()[0].source_link(),
        "https://example.com/full-size-nocturne.jpg"
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn desktop_shell_suggests_artwork_metadata_through_active_vault() {
    let root = temp_path("desktop-artwork-ai-vault");
    let source_dir = temp_path("desktop-artwork-ai-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"desktop painting bytes").expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let saved = shell
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Unknown Creator".to_string()),
            year: Some("Unknown Year".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference".to_string()),
        })
        .expect("save artwork");

    let provider = FakeProvider::new(AiProviderResponse {
        summary: None,
        tags: Vec::new(),
        suggestions: vec![AiMetadataSuggestion {
            field: "creator".to_string(),
            suggested_value: "Jane Painter".to_string(),
            confidence: 0.44,
            provenance: "fake-vision:signature".to_string(),
        }],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 4,
    });

    let enrichment = shell
        .suggest_artwork_metadata_with_ai(saved.id(), AiBudgetMode::Standard, &provider)
        .expect("suggest artwork metadata");

    assert_eq!(enrichment.staged_suggestions().len(), 1);
    assert_eq!(
        provider.last_request().image_bytes,
        Some(b"desktop painting bytes".to_vec())
    );
    assert_eq!(provider.last_request().cleaned_text, None);

    let details = shell.item_details(saved.id()).expect("read details");
    assert_eq!(details.metadata_suggestions()[0].field(), "creator");

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[derive(Debug)]
struct FakeProvider {
    response: AiProviderResponse,
    requests: RefCell<Vec<AiProviderRequest>>,
}

impl FakeProvider {
    fn new(response: AiProviderResponse) -> Self {
        Self {
            response,
            requests: RefCell::new(Vec::new()),
        }
    }

    fn last_request(&self) -> AiProviderRequest {
        self.requests
            .borrow()
            .last()
            .expect("provider request")
            .clone()
    }
}

impl AiProvider for FakeProvider {
    fn enrich(&self, request: AiProviderRequest) -> AiProviderResponse {
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
