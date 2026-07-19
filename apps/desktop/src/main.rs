use std::sync::Mutex;

use gruenes_gewolbe_desktop::{
    ActiveVaultView, AddArtworkFilesCommand, DesktopStartupView, OpenVaultView, SavedItemView,
    TauriCommandState, WorkbenchSnapshotCommand, WorkbenchSnapshotView,
};
use tauri::{Manager, State};

type CommandState = Mutex<TauriCommandState>;

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
    state: State<'_, CommandState>,
) -> Result<Vec<SavedItemView>, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .add_artwork_files(AddArtworkFilesCommand {
            source_files,
            creator,
            year,
            saving_reason,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn workbench_snapshot(
    artwork_sort: String,
    selected_item_id: Option<String>,
    state: State<'_, CommandState>,
) -> Result<WorkbenchSnapshotView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .workbench_snapshot(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort,
            search_query: None,
            selected_item_id,
        })
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_state_dir = app.path().app_data_dir()?;
            app.manage(Mutex::new(TauriCommandState::with_app_state_dir(
                app_state_dir,
            )));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            create_vault,
            open_vault,
            confirm_vault_repair,
            cancel_vault_repair,
            add_artwork_files,
            workbench_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Gruenes Gewoelbe");
}
