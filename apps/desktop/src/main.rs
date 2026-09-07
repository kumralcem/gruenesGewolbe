use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gruenes_gewolbe_core::{
    AiProvider, AiProviderRequest, AiProviderResponse, SourceExtraction, SourceExtractionRequest,
    SourceExtractor,
};

use gruenes_gewolbe_desktop::{
    source_extractor, ActiveVaultView, AddArtworkFilesCommand, CaptureIdeaCommand,
    CaptureSourceLinkCommand, ConfirmItemFolderRenameCommand, CopiedImageCommand,
    DesktopStartupView, DuplicateCandidateResolutionView, IdeaSourceContentView,
    ImportRunSummaryView, ItemDetailsView, ItemRecordSaveView, ManualFallbackCaptureCommand,
    OpenAiProviderConfig, OpenAiProviderStatusView, OpenAiResponsesProvider, OpenVaultView,
    PermanentDeletionView, ResolveDuplicateCandidateCommand, ResolveReviewReasonCommand,
    RunPaintingsImportCommand, SaveItemRecordCommand, SavedItemView, SelectedFileImportSummaryView,
    SourceCaptureResultView, SummarizeIdeaSourceView, TauriCommandState, ThumbnailPreparationView,
    WorkbenchSnapshotCommand, WorkbenchSnapshotView,
};
use tauri::{Emitter, Manager, State};

type CommandState = Arc<Mutex<TauriCommandState>>;
type ImportCancellation = Arc<AtomicBool>;

struct CompletedExtraction(SourceExtraction);

impl SourceExtractor for CompletedExtraction {
    fn extract(&self, _request: SourceExtractionRequest) -> SourceExtraction {
        self.0.clone()
    }
}

struct CompletedAiResponse(AiProviderResponse);

impl AiProvider for CompletedAiResponse {
    fn enrich(&self, _request: AiProviderRequest) -> AiProviderResponse {
        self.0.clone()
    }

    fn cost_estimate_is_known(&self) -> bool {
        false
    }
}

fn summarize_live(
    state: &CommandState,
    id: String,
    budget_mode: &str,
    expected_active_root: Option<&Path>,
) -> SummarizeIdeaSourceView {
    summarize_live_with(
        state,
        id,
        budget_mode,
        expected_active_root,
        |config, cleaned_text, budget| {
            OpenAiResponsesProvider::new(config)?.summarize(cleaned_text, budget)
        },
    )
}

fn summarize_live_with<F>(
    state: &CommandState,
    id: String,
    budget_mode: &str,
    expected_active_root: Option<&Path>,
    summarize: F,
) -> SummarizeIdeaSourceView
where
    F: FnOnce(
        OpenAiProviderConfig,
        &str,
        gruenes_gewolbe_core::AiBudgetMode,
    ) -> Result<AiProviderResponse, String>,
{
    let (config, source_before, active_root, record_revision) = match state.lock() {
        Ok(guard) => {
            let active_root = match guard.active_vault_root() {
                Some(root) => root,
                None => {
                    return SummarizeIdeaSourceView::Failed {
                        reason: Some("no Active Vault is open".to_string()),
                    }
                }
            };
            if expected_active_root.is_some_and(|expected| expected != active_root) {
                return SummarizeIdeaSourceView::Failed {
                    reason: Some(
                        "Active Vault changed before summary generation started".to_string(),
                    ),
                };
            }
            let details = match guard.get_item_details(id.clone()) {
                Ok(details) => details,
                Err(error) => {
                    return SummarizeIdeaSourceView::Failed {
                        reason: Some(error.to_string()),
                    }
                }
            };
            match (
                guard.openai_provider_status(),
                guard.read_idea_source(id.clone()),
            ) {
                (status, _) if !status.configured => {
                    return SummarizeIdeaSourceView::Unavailable {
                        reason: Some("Configure OpenAI to generate a summary".to_string()),
                    }
                }
                (_, Ok(source)) => match guard.shell_openai_provider_config() {
                    Ok(config) => (config, source, active_root, details.record_revision),
                    Err(error) => {
                        return SummarizeIdeaSourceView::Failed {
                            reason: Some(error.to_string()),
                        }
                    }
                },
                (_, Err(error)) => {
                    return SummarizeIdeaSourceView::Failed {
                        reason: Some(error.to_string()),
                    }
                }
            }
        }
        Err(_) => {
            return SummarizeIdeaSourceView::Failed {
                reason: Some("desktop state is unavailable".to_string()),
            }
        }
    };
    let budget = match budget_mode {
        "cheap" => gruenes_gewolbe_core::AiBudgetMode::Cheap,
        "standard" => gruenes_gewolbe_core::AiBudgetMode::Standard,
        "deep" => gruenes_gewolbe_core::AiBudgetMode::Deep,
        "off" => {
            return SummarizeIdeaSourceView::Skipped {
                reason: Some("AI budget is off".to_string()),
            }
        }
        _ => {
            return SummarizeIdeaSourceView::Failed {
                reason: Some("unsupported AI budget mode".to_string()),
            }
        }
    };
    let response = match summarize(config, &source_before.cleaned_text, budget) {
        Ok(response) => response,
        Err(reason) => {
            return SummarizeIdeaSourceView::Failed {
                reason: Some(reason),
            }
        }
    };
    let guard = match state.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return SummarizeIdeaSourceView::Failed {
                reason: Some("desktop state is unavailable".to_string()),
            }
        }
    };
    let current_root = guard.active_vault_root();
    let current_details = guard.get_item_details(id.clone());
    match (guard.read_idea_source(id.clone()), current_details) {
        (Ok(current), Ok(details))
            if current_root.as_deref() == Some(active_root.as_path())
                && details.record_revision == record_revision
                && current.cleaned_text == source_before.cleaned_text => guard
            .summarize_idea_source(id, budget_mode, &CompletedAiResponse(response))
            .unwrap_or_else(|error| SummarizeIdeaSourceView::Failed { reason: Some(error.to_string()) }),
        (Ok(_), Ok(_)) => SummarizeIdeaSourceView::Failed { reason: Some("Active Vault or Item Record changed while its summary was being generated; retry to summarize the current source".to_string()) },
        (Err(error), _) | (_, Err(error)) => SummarizeIdeaSourceView::Failed { reason: Some(error.to_string()) },
    }
}

fn decode_media_path(encoded_path: &str) -> Result<PathBuf, String> {
    let bytes = encoded_path
        .strip_prefix('/')
        .unwrap_or(encoded_path)
        .as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let pair = bytes
                .get(index + 1..index + 3)
                .ok_or_else(|| "invalid media path encoding".to_string())?;
            let pair =
                std::str::from_utf8(pair).map_err(|_| "invalid media path encoding".to_string())?;
            decoded.push(
                u8::from_str_radix(pair, 16)
                    .map_err(|_| "invalid media path encoding".to_string())?,
            );
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded)
        .map(PathBuf::from)
        .map_err(|_| "media path is not valid UTF-8".to_string())
}

fn read_active_vault_thumbnail(
    state: &CommandState,
    encoded_path: &str,
) -> Result<Vec<u8>, String> {
    let requested = decode_media_path(encoded_path)?;
    let active_root = state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .active_vault_root()
        .ok_or_else(|| "no Active Vault is open".to_string())?;
    let thumbnails = active_root.join(".gruenesgewolbe").join("thumbnails");
    let canonical_thumbnails = thumbnails
        .canonicalize()
        .map_err(|_| "Thumbnail Preview directory is unavailable".to_string())?;
    let canonical_requested = requested
        .canonicalize()
        .map_err(|_| "Thumbnail Preview is unavailable".to_string())?;
    if !canonical_requested.starts_with(&canonical_thumbnails)
        || !Path::new(&canonical_requested).is_file()
        || !canonical_requested
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err("media path is outside the Active Vault Thumbnail Previews".to_string());
    }
    std::fs::read(canonical_requested).map_err(|_| "Thumbnail Preview is unavailable".to_string())
}

#[tauri::command]
fn startup(state: State<'_, CommandState>) -> Result<DesktopStartupView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .startup()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_vault(root: String, state: State<'_, CommandState>) -> Result<ActiveVaultView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .create_vault(root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_vault(root: String, state: State<'_, CommandState>) -> Result<OpenVaultView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .open_vault(root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn confirm_vault_repair(
    root: String,
    state: State<'_, CommandState>,
) -> Result<ActiveVaultView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .confirm_vault_repair(root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_vault_repair(root: String, state: State<'_, CommandState>) -> Result<(), String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .cancel_vault_repair(root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_artwork_files(
    source_files: Vec<String>,
    creator: Option<String>,
    year: Option<String>,
    saving_reason: Option<String>,
    import_exact_duplicates: bool,
    state: State<'_, CommandState>,
) -> Result<SelectedFileImportSummaryView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .add_artwork_files(AddArtworkFilesCommand {
            source_files,
            creator,
            year,
            saving_reason,
            import_exact_duplicates,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn run_paintings_import(
    source_folder: String,
    creator: Option<String>,
    year: Option<String>,
    saving_reason: Option<String>,
    import_exact_duplicates: bool,
    app: tauri::AppHandle,
    state: State<'_, CommandState>,
    cancellation: State<'_, ImportCancellation>,
) -> Result<ImportRunSummaryView, String> {
    let state = Arc::clone(state.inner());
    let cancellation = Arc::clone(cancellation.inner());
    cancellation.store(false, Ordering::Release);
    tauri::async_runtime::spawn_blocking(move || {
        state
            .lock()
            .map_err(|_| "desktop state is unavailable".to_string())?
            .run_paintings_import(
                RunPaintingsImportCommand {
                    source_folder,
                    creator,
                    year,
                    saving_reason,
                    import_exact_duplicates,
                },
                |progress| {
                    let _ = app.emit("import-progress", progress);
                    cancellation.load(Ordering::Acquire)
                },
            )
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("import task failed: {error}"))?
}

#[tauri::command]
fn cancel_paintings_import(cancellation: State<'_, ImportCancellation>) {
    cancellation.store(true, Ordering::Release);
}

#[tauri::command]
async fn capture_source_link(
    source_link: String,
    title: String,
    saving_reason: Option<String>,
    state: State<'_, CommandState>,
) -> Result<SourceCaptureResultView, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let extractor = source_extractor::BoundedSourceExtractor::new()?;
        let active_root = state
            .lock()
            .map_err(|_| "desktop state is unavailable".to_string())?
            .active_vault_root()
            .ok_or_else(|| "no Active Vault is open".to_string())?;
        let extraction = extractor.extract(SourceExtractionRequest {
            source_link: source_link.clone(),
        });
        let guard = state
            .lock()
            .map_err(|_| "desktop state is unavailable".to_string())?;
        guard
            .capture_source_link(
                CaptureSourceLinkCommand {
                    source_link,
                    title,
                    saving_reason,
                },
                &CompletedExtraction(extraction),
                Some(active_root.as_path()),
            )
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("capture task failed: {error}"))?
}

#[tauri::command]
async fn capture_idea_source(
    source_link: String,
    title: String,
    saving_reason: Option<String>,
    copied_text: Option<String>,
    state: State<'_, CommandState>,
) -> Result<serde_json::Value, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let extractor = source_extractor::BoundedSourceExtractor::new()?;
        let active_root = state.lock().map_err(|_| "desktop state is unavailable".to_string())?
            .active_vault_root().ok_or_else(|| "no Active Vault is open".to_string())?;
        let extraction = if copied_text.as_deref().is_some_and(|text| !text.trim().is_empty()) {
            None
        } else {
            Some(extractor.extract_idea(SourceExtractionRequest { source_link: source_link.clone() }))
        };
        let result = state.lock().map_err(|_| "desktop state is unavailable".to_string())?
            .capture_idea_source(
                CaptureIdeaCommand { source_link, title, saving_reason, copied_text },
                &CompletedExtraction(extraction.unwrap_or(SourceExtraction::NeedsManualFallback { reason: "pasted text".to_string() })),
                Some(active_root.as_path()),
            ).map_err(|error| error.to_string())?;
        match result {
            SourceCaptureResultView::Captured { item } => {
                let summary = summarize_live(&state, item.id.clone(), "standard", Some(active_root.as_path()));
                Ok(serde_json::json!({
                    "status": "captured",
                    "item": item,
                    "summary_status": match &summary {
                        SummarizeIdeaSourceView::Generated { .. } => "generated",
                        SummarizeIdeaSourceView::Skipped { .. } => "skipped",
                        _ => "unavailable",
                    },
                    "summary": match summary { SummarizeIdeaSourceView::Generated { summary } => Some(summary), _ => None }
                }))
            }
            fallback => serde_json::to_value(fallback).map_err(|error| error.to_string()),
        }
    }).await.map_err(|error| format!("Idea Source capture task failed: {error}"))?
}

#[tauri::command]
fn get_item_details(
    id: String,
    state: State<'_, CommandState>,
) -> Result<gruenes_gewolbe_desktop::ItemDetailsView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .get_item_details(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_idea_source(
    id: String,
    state: State<'_, CommandState>,
) -> Result<IdeaSourceContentView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .read_idea_source(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn summarize_idea_source(
    id: String,
    budget_mode: String,
    state: State<'_, CommandState>,
) -> Result<SummarizeIdeaSourceView, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || Ok(summarize_live(&state, id, &budget_mode, None)))
        .await
        .map_err(|error| format!("summary task failed: {error}"))?
}

#[tauri::command]
fn configure_openai_provider(
    api_key: String,
    model: String,
    state: State<'_, CommandState>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .configure_openai_provider(OpenAiProviderConfig { api_key, model })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn openai_provider_status(
    state: State<'_, CommandState>,
) -> Result<OpenAiProviderStatusView, String> {
    Ok(state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .openai_provider_status())
}

#[tauri::command]
fn capture_manual_fallback(
    source_link: String,
    title: String,
    saving_reason: Option<String>,
    copied_text: Option<String>,
    copied_image_file_name: Option<String>,
    copied_image_bytes: Option<Vec<u8>>,
    state: State<'_, CommandState>,
) -> Result<SavedItemView, String> {
    let copied_image = match (copied_image_file_name, copied_image_bytes) {
        (Some(file_name), Some(bytes)) if !bytes.is_empty() => {
            Some(CopiedImageCommand { file_name, bytes })
        }
        _ => None,
    };
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .capture_manual_fallback(ManualFallbackCaptureCommand {
            source_link,
            title,
            saving_reason,
            copied_text,
            copied_image,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn workbench_snapshot(
    artwork_sort: String,
    selected_item_id: Option<String>,
    search_query: Option<String>,
    state: State<'_, CommandState>,
) -> Result<WorkbenchSnapshotView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .workbench_snapshot(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort,
            search_query,
            selected_item_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn refresh_workbench(
    artwork_sort: String,
    selected_item_id: Option<String>,
    search_query: Option<String>,
    state: State<'_, CommandState>,
) -> Result<WorkbenchSnapshotView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .refresh_workbench(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort,
            search_query,
            selected_item_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn prepare_thumbnail_previews(
    limit: usize,
    state: State<'_, CommandState>,
) -> Result<ThumbnailPreparationView, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        state
            .lock()
            .map_err(|_| "desktop state is unavailable".to_string())?
            .prepare_thumbnail_previews(limit)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("thumbnail task failed: {error}"))?
}

#[tauri::command]
fn activity_log_path(state: State<'_, CommandState>) -> Result<String, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .activity_log_path()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn move_item_to_trash(id: String, state: State<'_, CommandState>) -> Result<SavedItemView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .move_item_to_trash(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_trashed_item(
    id: String,
    state: State<'_, CommandState>,
) -> Result<SavedItemView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .restore_trashed_item(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn permanently_delete_trashed_item(
    id: String,
    confirmed_id: String,
    state: State<'_, CommandState>,
) -> Result<PermanentDeletionView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .permanently_delete_trashed_item(id, confirmed_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_item_record(
    id: String,
    expected_revision: String,
    overwrite_conflict: bool,
    title: String,
    creator: String,
    year: String,
    saving_reason: String,
    summary: String,
    tags: Vec<String>,
    state: State<'_, CommandState>,
) -> Result<ItemRecordSaveView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .save_item_record(SaveItemRecordCommand {
            id,
            expected_revision,
            overwrite_conflict,
            title,
            creator,
            year,
            saving_reason,
            summary,
            tags,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn resolve_review_reason(
    item_id: String,
    reason_id: String,
    expected_revision: String,
    action: String,
    correction: Option<String>,
    state: State<'_, CommandState>,
) -> Result<ItemDetailsView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .resolve_review_reason(ResolveReviewReasonCommand {
            item_id,
            reason_id,
            expected_revision,
            action,
            correction,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn resolve_duplicate_candidate(
    item_id: String,
    reason_id: String,
    expected_revision: String,
    action: String,
    state: State<'_, CommandState>,
) -> Result<DuplicateCandidateResolutionView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .resolve_duplicate_candidate(ResolveDuplicateCandidateCommand {
            item_id,
            reason_id,
            expected_revision,
            action,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn confirm_item_folder_rename(
    id: String,
    current_path: String,
    proposed_path: String,
    state: State<'_, CommandState>,
) -> Result<ItemDetailsView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .confirm_item_folder_rename(ConfirmItemFolderRenameCommand {
            id,
            current_path,
            proposed_path,
        })
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("vault-media", |context, request| {
            let state = context.app_handle().state::<CommandState>();
            match read_active_vault_thumbnail(state.inner(), request.uri().path()) {
                Ok(bytes) => tauri::http::Response::builder()
                    .header(tauri::http::header::CONTENT_TYPE, "image/png")
                    .body(bytes)
                    .expect("valid Thumbnail Preview response"),
                Err(message) => tauri::http::Response::builder()
                    .status(tauri::http::StatusCode::FORBIDDEN)
                    .header(
                        tauri::http::header::CONTENT_TYPE,
                        "text/plain; charset=utf-8",
                    )
                    .body(message.into_bytes())
                    .expect("valid media refusal response"),
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_state_dir = app.path().app_data_dir()?;
            app.manage(Arc::new(Mutex::new(TauriCommandState::with_app_state_dir(
                app_state_dir,
            ))));
            app.manage(Arc::new(AtomicBool::new(false)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            create_vault,
            open_vault,
            confirm_vault_repair,
            cancel_vault_repair,
            add_artwork_files,
            run_paintings_import,
            cancel_paintings_import,
            capture_source_link,
            capture_idea_source,
            capture_manual_fallback,
            get_item_details,
            read_idea_source,
            summarize_idea_source,
            configure_openai_provider,
            openai_provider_status,
            workbench_snapshot,
            refresh_workbench,
            prepare_thumbnail_previews,
            activity_log_path,
            move_item_to_trash,
            restore_trashed_item,
            permanently_delete_trashed_item,
            save_item_record,
            resolve_review_reason,
            resolve_duplicate_candidate,
            confirm_item_folder_rename
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Gruenes Gewoelbe");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn media_protocol_reads_only_the_current_active_vault_thumbnails() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gruenes-gewolbe-media-{unique}"));
        let state = Arc::new(Mutex::new(TauriCommandState::with_app_state_dir(
            root.join("app-state"),
        )));
        let first = root.join("first");
        let second = root.join("second");
        state
            .lock()
            .expect("desktop state")
            .create_vault(first.to_string_lossy().into_owned())
            .expect("create first Vault");
        let first_thumbnail = first.join(".gruenesgewolbe/thumbnails/first.png");
        std::fs::create_dir_all(first_thumbnail.parent().expect("thumbnail parent"))
            .expect("create thumbnail directory");
        std::fs::write(&first_thumbnail, b"first").expect("write first thumbnail");
        let non_preview = first_thumbnail.with_extension("txt");
        std::fs::write(&non_preview, b"not an image").expect("write non-preview file");
        let encoded_first = first_thumbnail.to_string_lossy().replace('/', "%2F");
        assert_eq!(
            read_active_vault_thumbnail(&state, &format!("/{encoded_first}"))
                .expect("read Active Vault thumbnail"),
            b"first"
        );
        assert!(read_active_vault_thumbnail(
            &state,
            &format!("/{}", non_preview.to_string_lossy().replace('/', "%2F")),
        )
        .is_err());

        state
            .lock()
            .expect("desktop state")
            .create_vault(second.to_string_lossy().into_owned())
            .expect("create second Vault");
        assert!(read_active_vault_thumbnail(&state, &format!("/{encoded_first}")).is_err());
        assert!(read_active_vault_thumbnail(
            &state,
            &format!(
                "/{}",
                first
                    .join("vault.toml")
                    .to_string_lossy()
                    .replace('/', "%2F")
            ),
        )
        .is_err());

        std::fs::remove_dir_all(root).expect("clean fixture");
    }

    #[test]
    fn summary_is_not_applied_after_switching_to_an_identical_copied_vault() {
        let (root, state, id, _) = summary_fixture("summary-stale-vault");
        let first = state
            .lock()
            .expect("desktop state")
            .active_vault_root()
            .expect("active vault");
        let second = root.join("copied-vault");
        copy_directory(&first, &second);
        let switch_state = Arc::clone(&state);

        let result = summarize_live_with(
            &state,
            id.clone(),
            "standard",
            Some(first.as_path()),
            move |_, _, _| {
                switch_state
                    .lock()
                    .expect("desktop state")
                    .open_vault(second.to_string_lossy().into_owned())
                    .expect("switch to copied vault");
                Ok(summary_response())
            },
        );

        assert!(matches!(result, SummarizeIdeaSourceView::Failed { .. }));
        assert_eq!(
            state
                .lock()
                .expect("desktop state")
                .read_idea_source(id)
                .expect("read copied source")
                .summary,
            None
        );
        fs::remove_dir_all(root).expect("clean fixture");
    }

    #[test]
    fn summary_is_not_applied_when_preserved_source_changes_during_provider_call() {
        let (root, state, id, item_folder) = summary_fixture("summary-stale-source");
        let source_path = item_folder.join("source-copies/cleaned-text.md");
        let result = summarize_live_with(&state, id.clone(), "standard", None, move |_, _, _| {
            fs::write(source_path, "A newer preserved source.\n").expect("replace source");
            Ok(summary_response())
        });

        assert!(matches!(result, SummarizeIdeaSourceView::Failed { .. }));
        let source = state
            .lock()
            .expect("desktop state")
            .read_idea_source(id)
            .expect("read current source");
        assert_eq!(source.cleaned_text, "A newer preserved source.");
        assert_eq!(source.summary, None);
        fs::remove_dir_all(root).expect("clean fixture");
    }

    #[test]
    fn summary_is_not_applied_when_item_record_changes_during_provider_call() {
        let (root, state, id, item_folder) = summary_fixture("summary-stale-record");
        let record_path = item_folder.join("record.md");
        let result = summarize_live_with(&state, id.clone(), "standard", None, move |_, _, _| {
            let record = fs::read_to_string(&record_path).expect("read record");
            fs::write(
                &record_path,
                record.replace("title: Source", "title: Revised source"),
            )
            .expect("revise record");
            Ok(summary_response())
        });

        assert!(matches!(result, SummarizeIdeaSourceView::Failed { .. }));
        assert_eq!(
            state
                .lock()
                .expect("desktop state")
                .read_idea_source(id)
                .expect("read source")
                .summary,
            None
        );
        fs::remove_dir_all(root).expect("clean fixture");
    }

    #[test]
    fn provider_failure_keeps_the_preserved_source_without_a_summary() {
        let (root, state, id, _) = summary_fixture("summary-provider-failure");
        let result = summarize_live_with(&state, id.clone(), "standard", None, |_, text, _| {
            assert_eq!(text, "Original preserved source.");
            Err("simulated provider failure".to_string())
        });

        assert!(matches!(result, SummarizeIdeaSourceView::Failed { .. }));
        let source = state
            .lock()
            .expect("desktop state")
            .read_idea_source(id)
            .expect("read source after failure");
        assert_eq!(source.cleaned_text, "Original preserved source.");
        assert_eq!(source.summary, None);
        fs::remove_dir_all(root).expect("clean fixture");
    }

    fn summary_fixture(name: &str) -> (PathBuf, CommandState, String, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"));
        let state = Arc::new(Mutex::new(TauriCommandState::with_app_state_dir(
            root.join("app-state"),
        )));
        let saved = {
            let mut guard = state.lock().expect("desktop state");
            guard
                .create_vault(root.join("vault").to_string_lossy().into_owned())
                .expect("create vault");
            guard
                .configure_openai_provider(OpenAiProviderConfig {
                    api_key: "test-key".to_string(),
                    model: "test-model".to_string(),
                })
                .expect("configure provider");
            guard
                .capture_idea(CaptureIdeaCommand {
                    source_link: "https://example.com/source".to_string(),
                    title: "Source".to_string(),
                    saving_reason: None,
                    copied_text: Some("Original preserved source.".to_string()),
                })
                .expect("capture source")
        };
        (root, state, saved.id, PathBuf::from(saved.item_folder))
    }

    fn summary_response() -> AiProviderResponse {
        AiProviderResponse {
            summary: Some("Generated summary.".to_string()),
            tags: Vec::new(),
            suggestions: Vec::new(),
            better_file_candidates: Vec::new(),
            estimated_cost_cents: 0,
        }
    }

    fn copy_directory(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).expect("create copied directory");
        for entry in fs::read_dir(source).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let target = destination.join(entry.file_name());
            if entry.file_type().expect("read source type").is_dir() {
                copy_directory(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).expect("copy source file");
            }
        }
    }
}
