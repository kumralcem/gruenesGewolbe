use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{
    AddArtworkItem, ArtworkImportMetadata, ReviewReasonAction, ReviewReasonResolution, Vault,
};

#[test]
fn review_status_is_derived_from_individually_resolved_reasons() {
    let root = temp_path("individual-review-reasons-vault");
    let source_dir = temp_path("individual-review-reasons-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("untitled.jpg");
    fs::write(&source_file, b"artwork bytes").expect("write artwork source");

    let vault = Vault::create(&root).expect("create vault");
    let item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "Untitled".to_string(),
            saving_reason: None,
        })
        .expect("add artwork");

    let details = vault.item_details(item.id()).expect("open item details");
    assert_eq!(details.review_status(), "needs-review");
    assert_eq!(details.review_reasons().len(), 2);
    assert_eq!(details.review_reasons()[0].kind(), "unknown-metadata");
    assert_eq!(details.review_reasons()[0].target_field(), Some("creator"));
    assert!(!details.review_reasons()[0].evidence().is_empty());
    assert_eq!(details.review_reasons()[1].target_field(), Some("year"));

    let after_creator = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: details.review_reasons()[0].id().to_string(),
            expected_revision: details.record_revision().to_string(),
            action: ReviewReasonAction::Correct {
                value: "Jane Painter".to_string(),
            },
        })
        .expect("correct creator reason");
    assert_eq!(after_creator.creator(), "Jane Painter");
    assert_eq!(after_creator.review_status(), "needs-review");
    assert_eq!(after_creator.review_reasons().len(), 1);

    let reviewed = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: after_creator.review_reasons()[0].id().to_string(),
            expected_revision: after_creator.record_revision().to_string(),
            action: ReviewReasonAction::Dismiss,
        })
        .expect("dismiss year reason");
    assert_eq!(reviewed.review_status(), "reviewed");
    assert!(reviewed.review_reasons().is_empty());

    let record = fs::read_to_string(item.item_folder().join("record.md"))
        .expect("read persisted item record");
    assert!(record.contains("review_reasons: []"));
    assert!(record.contains("review_status: reviewed"));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn metadata_suggestions_are_accepted_edited_or_dismissed_offline_with_provenance() {
    let root = temp_path("metadata-suggestion-review-vault");
    let source_dir = temp_path("metadata-suggestion-review-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("study.jpg");
    fs::write(&source_file, b"study bytes").expect("write artwork source");

    let vault = Vault::create(&root).expect("create vault");
    let item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Unknown Creator".to_string()),
            year: Some("Unknown Year".to_string()),
            title: "Study".to_string(),
            saving_reason: None,
        })
        .expect("add artwork");
    let record_path = item.item_folder().join("record.md");
    let record = fs::read_to_string(&record_path).expect("read record");
    let record = record.replace(
        "review_reasons:\n- unknown-creator | unknown-metadata | creator | Creator is unknown | No creator metadata was supplied or inferred\n- unknown-year | unknown-metadata | year | Year is unknown | No year metadata was supplied or inferred",
        "review_reasons:\n- suggest-creator | metadata-suggestion | creator | Suggested creator Jane Painter | visual analysis\n- suggest-year | metadata-suggestion | year | Suggested year 1884 | catalog comparison\n- suggest-title | metadata-suggestion | title | Suggested title Nocturne | visual analysis",
    );
    fs::write(
        &record_path,
        format!(
            "{}\n## Metadata Suggestions\n\n- creator | Jane Painter | 0.72 | visual analysis\n- year | 1884 | 0.81 | catalog comparison\n- title | Nocturne | 0.65 | visual analysis\n",
            record.trim_end()
        ),
    )
    .expect("stage metadata suggestions");

    let details = vault.item_details(item.id()).expect("open suggestions");
    let accepted = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: "suggest-creator".to_string(),
            expected_revision: details.record_revision().to_string(),
            action: ReviewReasonAction::Accept,
        })
        .expect("accept creator suggestion");
    assert_eq!(accepted.creator(), "Jane Painter");
    assert_eq!(accepted.metadata_suggestions().len(), 2);
    assert_eq!(accepted.metadata_provenance()[0].field(), "creator");
    assert_eq!(
        accepted.metadata_provenance()[0].provenance(),
        "visual analysis"
    );

    let edited = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: "suggest-year".to_string(),
            expected_revision: accepted.record_revision().to_string(),
            action: ReviewReasonAction::Correct {
                value: "1885".to_string(),
            },
        })
        .expect("edit then accept year suggestion");
    assert_eq!(edited.year(), "1885");
    assert_eq!(edited.metadata_suggestions().len(), 1);
    assert_eq!(edited.metadata_provenance()[1].suggested_value(), "1885");
    assert!(edited.metadata_provenance()[1]
        .provenance()
        .contains("catalog comparison"));

    let dismissed = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: "suggest-title".to_string(),
            expected_revision: edited.record_revision().to_string(),
            action: ReviewReasonAction::Dismiss,
        })
        .expect("dismiss title suggestion");
    assert_eq!(dismissed.title(), "Study");
    assert!(dismissed.metadata_suggestions().is_empty());
    assert_eq!(dismissed.review_status(), "reviewed");

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn conflicting_import_metadata_is_an_independent_review_reason() {
    let root = temp_path("conflicting-import-metadata-vault");
    let source_dir = temp_path("conflicting-import-metadata-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("Jane Painter - 1884 - Nocturne.jpg");
    fs::write(&source_file, b"nocturne bytes").expect("write artwork source");

    let vault = Vault::create(&root).expect("create vault");
    let items = vault
        .add_artwork_files_with_metadata(
            [source_file],
            ArtworkImportMetadata {
                creator: Some("John Painter".to_string()),
                year: Some("1885".to_string()),
                saving_reason: None,
            },
        )
        .expect("import with conflicting metadata");
    let details = vault
        .item_details(items[0].id())
        .expect("open imported item");

    assert_eq!(details.review_reasons().len(), 2);
    assert!(details.review_reasons().iter().any(|reason| {
        reason.kind() == "conflicting-metadata"
            && reason.target_field() == Some("creator")
            && reason.evidence().contains("Jane Painter")
    }));
    assert!(details.review_reasons().iter().any(|reason| {
        reason.kind() == "conflicting-metadata"
            && reason.target_field() == Some("year")
            && reason.evidence().contains("1884")
    }));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn a_non_field_review_reason_can_be_corrected_with_a_resolution_note() {
    let root = temp_path("generic-review-correction-vault");
    let source_dir = temp_path("generic-review-correction-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("study.jpg");
    fs::write(&source_file, b"study bytes").expect("write artwork source");

    let vault = Vault::create(&root).expect("create vault");
    let item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "Study".to_string(),
            saving_reason: None,
        })
        .expect("add artwork");
    let record_path = item.item_folder().join("record.md");
    let record = fs::read_to_string(&record_path).expect("read record");
    fs::write(
        &record_path,
        record.replace(
            "review_reasons:\n- unknown-creator | unknown-metadata | creator | Creator is unknown | No creator metadata was supplied or inferred\n- unknown-year | unknown-metadata | year | Year is unknown | No year metadata was supplied or inferred",
            "review_reasons:\n- preview-fixed | thumbnail-preview-unavailable |  | Thumbnail Preview is unavailable | decoder rejected source",
        ),
    )
    .expect("stage preview reason");

    let details = vault.item_details(item.id()).expect("open review reason");
    let reviewed = vault
        .resolve_review_reason(ReviewReasonResolution {
            item_id: item.id().to_string(),
            reason_id: "preview-fixed".to_string(),
            expected_revision: details.record_revision().to_string(),
            action: ReviewReasonAction::Correct {
                value: "Replaced the damaged preserved file manually".to_string(),
            },
        })
        .expect("record generic correction");

    assert_eq!(reviewed.review_status(), "reviewed");
    let record = fs::read_to_string(record_path).expect("read corrected record");
    assert!(record.contains(
        "## Review Resolutions\n\n- preview-fixed | corrected | Replaced the damaged preserved file manually"
    ));

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
