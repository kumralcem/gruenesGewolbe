use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{DesktopShell, OpenAiProviderConfig};

#[test]
fn desktop_shell_stores_openai_provider_config_in_user_app_state_not_the_vault() {
    let root = temp_path("desktop-openai-config-vault");
    let app_state = temp_path("desktop-openai-config-app-state");
    let mut shell = DesktopShell::with_app_state_dir(&app_state);
    shell.create_vault(&root).expect("create active vault");

    shell
        .configure_openai_provider(OpenAiProviderConfig {
            api_key: "sk-test-key".to_string(),
            model: "gpt-4.1-mini".to_string(),
        })
        .expect("configure provider");

    let config = shell
        .openai_provider_config()
        .expect("read provider config");
    assert_eq!(config.api_key(), "sk-test-key");
    assert_eq!(config.model(), "gpt-4.1-mini");

    let app_state_config =
        fs::read_to_string(app_state.join("openai-provider.toml")).expect("app state config");
    assert!(app_state_config.contains("api_key = \"sk-test-key\""));
    assert!(app_state_config.contains("model = \"gpt-4.1-mini\""));
    assert!(!root.join("openai-provider.toml").exists());
    assert!(!root.join("openai-config.toml").exists());
    assert!(!vault_files_contain(&root, "sk-test-key"));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&app_state).expect("clean app state");
}

fn vault_files_contain(root: &PathBuf, needle: &str) -> bool {
    let mut pending = vec![root.clone()];
    while let Some(path) = pending.pop() {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };

        if metadata.is_dir() {
            let Ok(entries) = fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.flatten() {
                pending.push(entry.path());
            }
        } else if metadata.is_file()
            && fs::read_to_string(&path)
                .map(|contents| contents.contains(needle))
                .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
