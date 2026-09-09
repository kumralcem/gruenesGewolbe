use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{DesktopShell, OpenVaultResult};

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

#[test]
fn desktop_shell_reopens_the_last_valid_active_vault() {
    let root = temp_path("desktop-last-active-vault");
    let app_state = temp_path("desktop-last-active-app-state");

    let mut shell = DesktopShell::with_app_state_dir(&app_state);
    shell.create_vault(&root).expect("create active vault");
    drop(shell);

    let restored_shell = DesktopShell::with_app_state_dir(&app_state);

    assert_eq!(
        restored_shell
            .active_vault()
            .expect("restored active vault")
            .root(),
        root.as_path()
    );
    assert!(!root.join("last-active-vault.txt").exists());

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&app_state).expect("clean app state");
}

#[test]
fn desktop_shell_reports_an_unavailable_last_active_vault_without_switching() {
    let root = temp_path("desktop-missing-last-active-vault");
    let app_state = temp_path("desktop-missing-last-active-app-state");

    let mut shell = DesktopShell::with_app_state_dir(&app_state);
    shell.create_vault(&root).expect("create active vault");
    drop(shell);
    fs::remove_dir_all(&root).expect("remove remembered vault");

    let restored_shell = DesktopShell::with_app_state_dir(&app_state);
    let startup = restored_shell.startup_state().expect("startup state");

    assert!(startup.active_vault().is_none());
    assert_eq!(roots(startup.known_vaults().to_vec()), vec![root.clone()]);
    assert_eq!(
        startup.notice(),
        Some(format!(
            "last active vault is unavailable: {}",
            root.display()
        ))
        .as_deref()
    );

    fs::remove_dir_all(&app_state).expect("clean app state");
}

#[test]
fn desktop_shell_proposes_and_cancels_repair_without_activating_or_changing_the_vault() {
    let root = temp_path("desktop-repair-cancel");
    fs::create_dir_all(&root).expect("create incomplete vault root");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write vault config");
    let mut shell = DesktopShell::default();

    let OpenVaultResult::RepairRequired(proposal) = shell
        .request_open_vault(&root)
        .expect("request incomplete vault open")
    else {
        panic!("incomplete vault should require repair");
    };

    assert_eq!(proposal.root(), root.as_path());
    assert_eq!(
        proposal.directories(),
        &[root.join("subvaults"), root.join("collections")]
    );
    assert!(shell.active_vault().is_none());

    shell
        .cancel_vault_repair(&root)
        .expect("cancel proposed repair");

    assert!(!root.join("subvaults").exists());
    assert!(!root.join("collections").exists());
    assert!(shell.active_vault().is_none());

    fs::remove_dir_all(&root).expect("clean incomplete vault");
}

#[test]
fn desktop_shell_confirms_repair_and_opens_the_repaired_vault() {
    let root = temp_path("desktop-confirm-repair");
    fs::create_dir_all(root.join("subvaults")).expect("create existing subvaults");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write vault config");
    let mut shell = DesktopShell::default();
    let proposal = shell
        .request_open_vault(&root)
        .expect("request incomplete vault open");
    assert!(matches!(proposal, OpenVaultResult::RepairRequired(_)));

    let active = shell
        .confirm_vault_repair(&root)
        .expect("confirm proposed repair");

    assert_eq!(active.root(), root.as_path());
    assert!(root.join("collections").is_dir());
    assert_eq!(
        shell.active_vault().expect("active repaired vault").root(),
        root.as_path()
    );

    fs::remove_dir_all(&root).expect("clean repaired vault");
}

#[test]
fn desktop_startup_exposes_repair_for_a_remembered_incomplete_vault() {
    let root = temp_path("desktop-startup-repair");
    let app_state = temp_path("desktop-startup-repair-state");
    let mut shell = DesktopShell::with_app_state_dir(&app_state);
    shell.create_vault(&root).expect("create remembered vault");
    drop(shell);
    fs::remove_dir(root.join("collections")).expect("remove required directory");

    let restored_shell = DesktopShell::with_app_state_dir(&app_state);
    let startup = restored_shell.startup_state().expect("startup state");

    assert!(startup.active_vault().is_none());
    assert_eq!(
        startup
            .repair_proposal()
            .expect("startup repair proposal")
            .directories(),
        &[root.join("collections")]
    );
    assert!(startup.notice().is_none());
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean incomplete vault");
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
