use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_logs_item_detail_errors_with_enough_context_for_the_app() {
    let root = temp_path("desktop-error-activity-vault");
    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");

    let error = shell
        .item_details("missing-item")
        .expect_err("missing item should fail");
    assert_eq!(error.to_string(), "saved item not found: missing-item");

    let activity_log = fs::read_to_string(root.join(".gruenesgewolbe").join("activity-log.tsv"))
        .expect("activity log");
    assert!(activity_log
        .contains("\terror\titem-details\tmissing-item\tsaved item not found: missing-item\n"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
