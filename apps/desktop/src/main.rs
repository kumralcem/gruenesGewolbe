use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gruenes_gewolbe_desktop::{
    ActiveVaultView, AddArtworkFilesCommand, ConfirmItemFolderRenameCommand, DesktopStartupView,
    ImportRunSummaryView, ItemDetailsView, ItemRecordSaveView, OpenVaultView,
    DuplicateCandidateResolutionView, ResolveDuplicateCandidateCommand,
    ResolveReviewReasonCommand, RunPaintingsImportCommand, SaveItemRecordCommand,
    SavedItemView, SelectedFileImportSummaryView, TauriCommandState, WorkbenchSnapshotCommand,
    WorkbenchSnapshotView,
};
use tauri::{Emitter, Manager, State};

type CommandState = Arc<Mutex<TauriCommandState>>;
type ImportCancellation = Arc<AtomicBool>;

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
fn activity_log_path(state: State<'_, CommandState>) -> Result<String, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .activity_log_path()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn move_item_to_trash(id: String, state: State<'_, CommandState>) -> Result<SavedItemView, String> {
    state.lock().map_err(|_| "desktop state is unavailable".to_string())?.move_item_to_trash(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_trashed_item(id: String, state: State<'_, CommandState>) -> Result<SavedItemView, String> {
    state.lock().map_err(|_| "desktop state is unavailable".to_string())?.restore_trashed_item(id).map_err(|e| e.to_string())
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
    state.lock().map_err(|_| "desktop state is unavailable".to_string())?
        .resolve_duplicate_candidate(ResolveDuplicateCandidateCommand {
            item_id, reason_id, expected_revision, action,
        }).map_err(|error| error.to_string())
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
            activity_log_path,
            move_item_to_trash,
            restore_trashed_item,
            save_item_record,
            resolve_review_reason,
            resolve_duplicate_candidate,
            confirm_item_folder_rename
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Gruenes Gewoelbe");
}
