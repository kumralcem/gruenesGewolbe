use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, Vault};

#[test]
fn cli_resolves_a_duplicate_candidate_with_an_explicit_item_reason_and_decision() {
    let root = temp_path("cli-duplicate-vault");
    let source = temp_path("cli-duplicate-source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("one.jpg"), b"one").unwrap();
    fs::write(source.join("two.jpg"), b"two").unwrap();
    let vault = Vault::create(&root).unwrap();
    add(&vault, source.join("one.jpg"));
    let second = add(&vault, source.join("two.jpg"));
    let details = vault.item_details(second.id()).unwrap();
    let reason = details.review_reasons().iter().find(|reason| reason.kind() == "duplicate-candidate").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .args(["resolve-duplicate-candidate", root.to_str().unwrap(), second.id(), reason.id(), "keep-both"])
        .output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("resolved-duplicate-candidate\t{}\tkeep-both\treviewed", second.id())));
    assert!(Vault::open(&root).unwrap().review_queue().unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(source).unwrap();
}

fn add(vault: &Vault, source_file: PathBuf) -> gruenes_gewolbe_core::SavedItem {
    vault.add_artwork_item(AddArtworkItem { source_file, home_subvault: "Paintings".into(), creator: Some("Jane Painter".into()), year: Some("1884".into()), title: "Nocturne Study".into(), saving_reason: None }).unwrap()
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
