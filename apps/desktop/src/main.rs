use std::sync::Mutex;

use gruenes_gewolbe_desktop::{ActiveVaultView, DesktopStartupView, TauriCommandState};
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
fn open_vault(root: String, state: State<'_, CommandState>) -> Result<ActiveVaultView, String> {
    state
        .lock()
        .map_err(|_| "desktop state is unavailable".to_string())?
        .open_vault(root)
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
        .invoke_handler(tauri::generate_handler![startup, create_vault, open_vault])
        .run(tauri::generate_context!())
        .expect("failed to run Gruenes Gewoelbe");
}
