use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    AddArtworkItem, AiBudgetMode, AiMetadataSuggestion, AiProvider, AiProviderRequest,
    AiProviderResponse, BetterFileCandidate, ExtractedTextCapture, TagDefinition, Vault,
};

#[test]
fn idea_enrichment_uses_cleaned_text_budget_mode_and_persists_summary_tags_and_suggestions() {
    let root = temp_path("ai-idea-vault");
    let vault = Vault::create(&root).expect("create vault");
    let captured = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/essay".to_string(),
            title: "Essay source".to_string(),
            saving_reason: Some("Source for archive design".to_string()),
            cleaned_text: "Cleaned main content only.".to_string(),
        })
        .expect("capture extracted text");

    let provider = FakeProvider::new(AiProviderResponse {
        summary: Some("A concise archive design note.".to_string()),
        tags: vec!["archive design".to_string(), "source material".to_string()],
        suggestions: vec![AiMetadataSuggestion {
            field: "title".to_string(),
            suggested_value: "Archive design source".to_string(),
            confidence: 0.42,
            provenance: "fake-provider:title".to_string(),
        }],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 3,
    });

    let enrichment = vault
        .enrich_idea_with_ai(captured.id(), AiBudgetMode::Standard, &provider)
        .expect("enrich idea");

    assert_eq!(
        enrichment.accepted_tags(),
        vec!["archive design", "source material"]
    );
    assert_eq!(
        enrichment.accepted_summary(),
        Some("A concise archive design note.")
    );
    assert_eq!(enrichment.staged_suggestions().len(), 1);
    assert_eq!(enrichment.estimated_cost_cents(), 3);

    let request = provider.last_request();
    assert_eq!(request.budget_mode, AiBudgetMode::Standard);
    assert_eq!(
        request.cleaned_text,
        Some("Cleaned main content only.".to_string())
    );
    assert_eq!(request.raw_html, None);
    assert!(request.related_item_records.is_empty());
    assert_eq!(request.image_bytes, None);

    let details = vault.item_details(captured.id()).expect("read details");
    assert_eq!(details.tags(), vec!["archive design", "source material"]);
    assert_eq!(details.summary(), Some("A concise archive design note."));
    assert_eq!(details.metadata_suggestions().len(), 1);
    assert_eq!(details.metadata_suggestions()[0].field(), "title");
    assert_eq!(
        details.metadata_suggestions()[0].suggested_value(),
        "Archive design source"
    );
    assert_eq!(
        details.metadata_suggestions()[0].provenance(),
        "fake-provider:title"
    );

    let record = fs::read_to_string(captured.item_folder().join("record.md")).expect("read record");
    assert!(record.contains("## Summary\n\nA concise archive design note.\n"));
    assert!(record.contains(
        "## Metadata Suggestions\n\n- title | Archive design source | 0.42 | fake-provider:title\n"
    ));
    let cost_log =
        fs::read_to_string(root.join(".gruenesgewolbe").join("ai-cost-log.tsv")).expect("cost log");
    assert!(cost_log.contains(&format!(
        "\tai-enrichment\t{}\tstandard\t3\n",
        captured.id()
    )));
    let activity_log = fs::read_to_string(root.join(".gruenesgewolbe").join("activity-log.tsv"))
        .expect("activity log");
    assert!(activity_log.contains(&format!("\tai-cost\t{}\tstandard\t3\n", captured.id())));
    assert!(activity_log.contains(&format!(
        "\tenrichment\t{}\tstandard\t2\t1\n",
        captured.id()
    )));
    assert!(!root.join("openai-config.toml").exists());
    assert!(!record.contains("sk-"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn provider_with_unknown_cost_does_not_record_a_zero_cost_entry() {
    let root = temp_path("ai-unknown-cost-vault");
    let vault = Vault::create(&root).expect("create vault");
    let captured = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/essay".to_string(),
            title: "Essay source".to_string(),
            saving_reason: None,
            cleaned_text: "Cleaned main content only.".to_string(),
        })
        .expect("capture source");
    let provider = UnknownCostProvider;

    vault
        .enrich_idea_with_ai(captured.id(), AiBudgetMode::Standard, &provider)
        .expect("enrich idea");

    assert!(!root.join(".gruenesgewolbe/ai-cost-log.tsv").exists());
    let activity = fs::read_to_string(root.join(".gruenesgewolbe/activity-log.tsv"))
        .expect("read activity log");
    assert!(!activity.contains("\tai-cost\t"));
    assert!(activity.contains(&format!("\tenrichment\t{}\tstandard", captured.id())));
    fs::remove_dir_all(root).expect("clean vault");
}

#[test]
fn idea_enrichment_normalizes_ai_tags_through_the_vault_tag_registry() {
    let root = temp_path("ai-tag-registry-vault");
    let vault = Vault::create(&root).expect("create vault");
    vault
        .upsert_tag(TagDefinition {
            name: "night palette".to_string(),
            aliases: vec!["nocturne colors".to_string()],
            meaning: Some("Dark color references for night scenes".to_string()),
        })
        .expect("create tag registry entry");
    let captured = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/nocturne".to_string(),
            title: "Nocturne source".to_string(),
            saving_reason: Some("Palette source".to_string()),
            cleaned_text: "A note about night color studies.".to_string(),
        })
        .expect("capture extracted text");

    let provider = FakeProvider::new(AiProviderResponse {
        summary: None,
        tags: vec![
            "nocturne colors".to_string(),
            "archive design".to_string(),
            "night palette".to_string(),
        ],
        suggestions: Vec::new(),
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 1,
    });

    let enrichment = vault
        .enrich_idea_with_ai(captured.id(), AiBudgetMode::Cheap, &provider)
        .expect("enrich idea");

    assert_eq!(
        enrichment.accepted_tags(),
        vec!["archive design", "night palette"]
    );

    let details = vault.item_details(captured.id()).expect("read details");
    assert_eq!(details.tags(), vec!["archive design", "night palette"]);

    assert!(!details.tags().contains(&"nocturne colors"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn idea_enrichment_budget_off_skips_provider_and_leaves_item_unchanged() {
    let root = temp_path("ai-budget-off-vault");
    let vault = Vault::create(&root).expect("create vault");
    let captured = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/essay".to_string(),
            title: "Essay source".to_string(),
            saving_reason: Some("Source for archive design".to_string()),
            cleaned_text: "Cleaned main content only.".to_string(),
        })
        .expect("capture extracted text");
    let before_record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read record");

    let provider = FakeProvider::new(AiProviderResponse {
        summary: Some("Should not be used.".to_string()),
        tags: vec!["unused".to_string()],
        suggestions: vec![AiMetadataSuggestion {
            field: "title".to_string(),
            suggested_value: "Unused".to_string(),
            confidence: 0.99,
            provenance: "fake-provider:title".to_string(),
        }],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 99,
    });

    let enrichment = vault
        .enrich_idea_with_ai(captured.id(), AiBudgetMode::Off, &provider)
        .expect("skip enrichment");

    assert_eq!(enrichment.accepted_summary(), None);
    assert!(enrichment.accepted_tags().is_empty());
    assert!(enrichment.staged_suggestions().is_empty());
    assert_eq!(enrichment.estimated_cost_cents(), 0);
    assert_eq!(provider.request_count(), 0);

    let after_record =
        fs::read_to_string(captured.item_folder().join("record.md")).expect("read record");
    assert_eq!(after_record, before_record);
    assert!(!root
        .join(".gruenesgewolbe")
        .join("ai-cost-log.tsv")
        .exists());

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn artwork_metadata_suggestions_send_only_primary_image_bytes_and_stage_metadata() {
    let root = temp_path("ai-artwork-vault");
    let source_dir = temp_path("ai-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("jane-nocturne.jpg");
    fs::write(&source_file, b"painting image bytes").expect("write source painting");

    let vault = Vault::create(&root).expect("create vault");
    let saved = vault
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
        suggestions: vec![
            AiMetadataSuggestion {
                field: "creator".to_string(),
                suggested_value: "Jane Painter".to_string(),
                confidence: 0.47,
                provenance: "fake-vision:signature".to_string(),
            },
            AiMetadataSuggestion {
                field: "year".to_string(),
                suggested_value: "1884".to_string(),
                confidence: 0.38,
                provenance: "fake-vision:inscription".to_string(),
            },
        ],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 5,
    });

    let enrichment = vault
        .suggest_artwork_metadata_with_ai(saved.id(), AiBudgetMode::Deep, &provider)
        .expect("suggest artwork metadata");

    assert!(enrichment.accepted_tags().is_empty());
    assert_eq!(enrichment.accepted_summary(), None);
    assert_eq!(enrichment.staged_suggestions().len(), 2);
    assert_eq!(enrichment.estimated_cost_cents(), 5);

    let request = provider.last_request();
    assert_eq!(request.item_id, saved.id());
    assert_eq!(request.budget_mode, AiBudgetMode::Deep);
    assert_eq!(request.image_bytes, Some(b"painting image bytes".to_vec()));
    assert_eq!(request.cleaned_text, None);
    assert_eq!(request.raw_html, None);
    assert!(request.related_item_records.is_empty());

    let details = vault.item_details(saved.id()).expect("read details");
    assert_eq!(details.metadata_suggestions().len(), 2);
    assert_eq!(details.metadata_suggestions()[0].field(), "creator");
    assert_eq!(
        details.metadata_suggestions()[0].suggested_value(),
        "Jane Painter"
    );
    assert_eq!(details.metadata_suggestions()[1].field(), "year");
    assert_eq!(details.review_status(), "needs-review");
    assert!(details
        .review_reasons()
        .iter()
        .any(|reason| reason.kind() == "metadata-suggestion"
            && reason.target_field() == Some("creator")));

    let queue = vault.review_queue().expect("browse review queue");
    assert!(queue
        .iter()
        .any(|item| item.saved_item().id() == saved.id()));

    let record = fs::read_to_string(saved.item_folder().join("record.md")).expect("read record");
    assert!(record.contains(
        "## Metadata Suggestions\n\n- creator | Jane Painter | 0.47 | fake-vision:signature\n- year | 1884 | 0.38 | fake-vision:inscription\n"
    ));
    assert!(record.contains("creator: Unknown Creator"));
    assert!(record.contains("year: Unknown Year"));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn artwork_enrichment_accepts_high_confidence_unknown_metadata_and_stages_conflicts() {
    let root = temp_path("ai-accepted-metadata-vault");
    let source_dir = temp_path("ai-accepted-metadata-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("unknown-nocturne.jpg");
    fs::write(&source_file, b"painting image bytes").expect("write source painting");

    let vault = Vault::create(&root).expect("create vault");
    let saved = vault
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
        suggestions: vec![
            AiMetadataSuggestion {
                field: "creator".to_string(),
                suggested_value: "Jane Painter".to_string(),
                confidence: 0.96,
                provenance: "fake-vision:signature".to_string(),
            },
            AiMetadataSuggestion {
                field: "year".to_string(),
                suggested_value: "1884".to_string(),
                confidence: 0.94,
                provenance: "fake-vision:inscription".to_string(),
            },
            AiMetadataSuggestion {
                field: "title".to_string(),
                suggested_value: "Moonlit Harbor".to_string(),
                confidence: 0.95,
                provenance: "fake-vision:catalogue".to_string(),
            },
        ],
        better_file_candidates: Vec::new(),
        estimated_cost_cents: 6,
    });

    let enrichment = vault
        .suggest_artwork_metadata_with_ai(saved.id(), AiBudgetMode::Standard, &provider)
        .expect("suggest artwork metadata");

    assert_eq!(enrichment.accepted_metadata().len(), 2);
    assert_eq!(enrichment.accepted_metadata()[0].field(), "creator");
    assert_eq!(enrichment.accepted_metadata()[1].field(), "year");
    assert_eq!(enrichment.staged_suggestions().len(), 1);
    assert_eq!(enrichment.staged_suggestions()[0].field(), "title");

    let details = vault.item_details(saved.id()).expect("read details");
    assert_eq!(details.creator(), "Jane Painter");
    assert_eq!(details.year(), "1884");
    assert_eq!(details.title(), "Nocturne Study");
    assert_eq!(details.metadata_provenance().len(), 2);
    assert_eq!(details.metadata_suggestions().len(), 1);
    assert_eq!(details.metadata_suggestions()[0].field(), "title");
    assert_eq!(details.review_reasons().len(), 1);
    assert_eq!(details.review_reasons()[0].kind(), "metadata-suggestion");
    assert_eq!(details.review_reasons()[0].target_field(), Some("title"));
    assert_eq!(
        details.metadata_suggestions()[0].suggested_value(),
        "Moonlit Harbor"
    );

    let record = fs::read_to_string(saved.item_folder().join("record.md")).expect("read record");
    assert!(record.contains("creator: Jane Painter"));
    assert!(record.contains("year: '1884'"));
    assert!(record.contains("title: Nocturne Study"));
    assert!(record.contains(
        "## Metadata Provenance\n\n- creator | Jane Painter | 0.96 | fake-vision:signature\n- year | 1884 | 0.94 | fake-vision:inscription\n"
    ));
    assert!(record.contains(
        "## Metadata Suggestions\n\n- title | Moonlit Harbor | 0.95 | fake-vision:catalogue\n"
    ));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn precomputed_artwork_enrichment_is_rejected_after_item_record_changes() {
    let root = temp_path("ai-stale-artwork-vault");
    let source_dir = temp_path("ai-stale-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("stale.jpg");
    fs::write(&source_file, b"painting image bytes").expect("write source painting");
    let vault = Vault::create(&root).expect("create vault");
    let saved = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Unknown Creator".to_string()),
            year: Some("Unknown Year".to_string()),
            title: "Stale Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork");
    let revision = vault
        .item_details(saved.id())
        .expect("read details")
        .record_revision()
        .to_string();
    let record_path = saved.item_folder().join("record.md");
    let mut externally_edited = fs::read_to_string(&record_path).expect("read record");
    externally_edited.push_str("\nUser note written while research was running.\n");
    fs::write(&record_path, externally_edited).expect("edit record externally");

    let error = vault
        .apply_artwork_enrichment_response(
            saved.id(),
            &revision,
            AiBudgetMode::Standard,
            AiProviderResponse {
                summary: None,
                tags: vec!["night scene".to_string()],
                suggestions: Vec::new(),
                better_file_candidates: Vec::new(),
                estimated_cost_cents: 0,
            },
            false,
        )
        .expect_err("reject stale enrichment");
    assert!(error.to_string().contains("item record changed"));
    let record = fs::read_to_string(record_path).expect("read unchanged external edit");
    assert!(record.contains("User note written while research was running."));
    assert!(!record.contains("night scene"));

    fs::remove_dir_all(root).expect("clean temp vault");
    fs::remove_dir_all(source_dir).expect("clean source directory");
}

#[test]
fn artwork_enrichment_records_better_file_candidates_without_replacing_the_primary_file() {
    let root = temp_path("ai-better-file-vault");
    let source_dir = temp_path("ai-better-file-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("small-nocturne.jpg");
    fs::write(&source_file, b"small painting image bytes").expect("write source painting");

    let vault = Vault::create(&root).expect("create vault");
    let saved = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference".to_string()),
        })
        .expect("save artwork");
    let original_primary_file = vault
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

    let enrichment = vault
        .suggest_artwork_metadata_with_ai(saved.id(), AiBudgetMode::Standard, &provider)
        .expect("suggest better file candidate");

    assert_eq!(enrichment.better_file_candidates().len(), 1);
    assert_eq!(
        enrichment.better_file_candidates()[0].source_link(),
        "https://example.com/full-size-nocturne.jpg"
    );

    let details = vault.item_details(saved.id()).expect("read details");
    assert_eq!(details.primary_file(), original_primary_file.as_path());
    assert_eq!(details.better_file_candidates().len(), 1);
    assert_eq!(
        details.better_file_candidates()[0].reason(),
        "Higher resolution source image"
    );
    assert_eq!(
        fs::read(details.primary_file()).expect("read original primary file"),
        b"small painting image bytes"
    );

    let record = fs::read_to_string(saved.item_folder().join("record.md")).expect("read record");
    assert!(record.contains(
        "## Better File Candidates\n\n- https://example.com/full-size-nocturne.jpg | Higher resolution source image | fake-vision:source-page\n"
    ));
    assert!(record.contains("primary_file: files/small-nocturne.jpg"));

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

    fn request_count(&self) -> usize {
        self.requests.borrow().len()
    }
}

impl AiProvider for FakeProvider {
    fn enrich(&self, request: AiProviderRequest) -> AiProviderResponse {
        self.requests.borrow_mut().push(request);
        self.response.clone()
    }
}

struct UnknownCostProvider;

impl AiProvider for UnknownCostProvider {
    fn enrich(&self, _request: AiProviderRequest) -> AiProviderResponse {
        AiProviderResponse {
            summary: Some("Summary with unknown billed cost.".to_string()),
            tags: Vec::new(),
            suggestions: Vec::new(),
            better_file_candidates: Vec::new(),
            estimated_cost_cents: 0,
        }
    }

    fn cost_estimate_is_known(&self) -> bool {
        false
    }
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
