use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gruenes_gewolbe_desktop::{
    ActiveVaultView, AddArtworkFilesCommand, ConfirmItemFolderRenameCommand, DesktopStartupView,
    DuplicateCandidateResolutionView, ImportRunSummaryView, ItemDetailsView, ItemRecordSaveView,
    OpenVaultView, PermanentDeletionView, ResolveDuplicateCandidateCommand,
    ResolveReviewReasonCommand, RunPaintingsImportCommand, SaveItemRecordCommand, SavedItemView,
    SelectedFileImportSummaryView, TauriCommandState, ThumbnailPreparationView,
    WorkbenchSnapshotCommand, WorkbenchSnapshotView,
};
use tauri::{Emitter, Manager, State};

type CommandState = Arc<Mutex<TauriCommandState>>;
type ImportCancellation = Arc<AtomicBool>;

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
}
