use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_makes_the_active_vault_explicit_after_create_and_open() {
    let root = temp_path("desktop-active");
    let mut shell = DesktopShell::default();

    let created = shell.create_vault(&root).expect("create active vault");
    assert_eq!(created.root(), root.as_path());
    assert_eq!(
        shell.active_vault().expect("active vault").root(),
        root.as_path()
    );

    let mut reopened_shell = DesktopShell::default();
    let opened = reopened_shell.open_vault(&root).expect("open active vault");
    assert_eq!(opened.root(), root.as_path());
    assert_eq!(
        reopened_shell.active_vault().expect("active vault").root(),
        root.as_path()
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn desktop_shell_remembers_multiple_vault_roots_and_switches_the_active_vault() {
    let first_root = temp_path("desktop-first-vault");
    let second_root = temp_path("desktop-second-vault");
    let app_state = temp_path("desktop-vault-history-app-state");
    let mut shell = DesktopShell::with_app_state_dir(&app_state);

    shell
        .create_vault(&first_root)
        .expect("create first active vault");
    shell
        .create_vault(&second_root)
        .expect("create second active vault");

    assert_eq!(
        roots(shell.known_vaults().expect("known vaults")),
        vec![first_root.clone(), second_root.clone()]
    );
    assert_eq!(
        shell.active_vault().expect("active vault").root(),
        second_root.as_path()
    );

    let restored_shell = DesktopShell::with_app_state_dir(&app_state);
    assert_eq!(
        roots(
            restored_shell
                .known_vaults()
                .expect("restored known vaults")
        ),
        vec![first_root.clone(), second_root.clone()]
    );

    shell
        .switch_active_vault(&first_root)
        .expect("switch active vault");
    assert_eq!(
        shell.active_vault().expect("active vault").root(),
        first_root.as_path()
    );

    fs::remove_dir_all(&first_root).expect("clean first vault");
    fs::remove_dir_all(&second_root).expect("clean second vault");
    fs::remove_dir_all(&app_state).expect("clean app state");
}

fn roots(vaults: Vec<gruenes_gewolbe_desktop::KnownVault>) -> Vec<PathBuf> {
    vaults
        .into_iter()
        .map(|vault| vault.root().to_path_buf())
        .collect()
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
