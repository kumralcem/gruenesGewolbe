use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_inspects_an_item_record_as_structured_lines() {
    let root = temp_path("cli-inspect-vault");
    let source_dir = temp_path("cli-inspect-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let create = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("create")
        .arg(&root)
        .output()
        .expect("run create command");
    assert!(create.status.success());

    let import = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("import-paintings")
        .arg(&root)
        .arg(&source_dir)
        .output()
        .expect("run import command");
    assert!(import.status.success());

    let search = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("search")
        .arg(&root)
        .arg("nocturne")
        .output()
        .expect("run search command");
    assert!(search.status.success());
    let search_stdout = String::from_utf8(search.stdout).expect("search stdout");
    let item_id = search_stdout
        .lines()
        .next()
        .expect("search result")
        .split('\t')
        .nth(1)
        .expect("search result id");

    let inspect = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("inspect-item")
        .arg(&root)
        .arg(item_id)
        .output()
        .expect("run inspect command");
    assert!(inspect.status.success());

    let stdout = String::from_utf8(inspect.stdout).expect("inspect stdout");
    assert!(stdout.contains(&format!("item\t{item_id}\tPaintings\t")));
    assert!(stdout.contains("\tJane Painter - 1884 - Nocturne Study\n"));
    assert!(stdout.contains("field\ttitle\tNocturne Study\n"));
    assert!(stdout.contains("field\tcreator\tJane Painter\n"));
    assert!(stdout.contains("field\tyear\t1884\n"));
    assert!(stdout.contains("field\treview_status\tneeds-review\n"));
    assert!(stdout.contains("file\tprimary\t"));
    assert!(stdout.contains("Jane Painter - 1884 - Nocturne Study.jpg\n"));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn cli_logs_structured_errors_for_failed_item_inspection() {
    let root = temp_path("cli-inspect-error-vault");

    let create = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("create")
        .arg(&root)
        .output()
        .expect("run create command");
    assert!(create.status.success());

    let inspect = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("inspect-item")
        .arg(&root)
        .arg("missing-item")
        .output()
        .expect("run inspect command");
    assert!(!inspect.status.success());
    assert_eq!(
        String::from_utf8(inspect.stderr).expect("inspect stderr"),
        "error\tinspect-item\tsaved item not found: missing-item\n"
    );

    let activity_log = fs::read_to_string(root.join(".gruenesgewolbe").join("activity-log.tsv"))
        .expect("activity log");
    assert!(activity_log
        .contains("\terror\tinspect-item\tmissing-item\tsaved item not found: missing-item\n"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
