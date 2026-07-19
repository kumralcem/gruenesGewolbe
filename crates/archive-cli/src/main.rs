use std::env;
use std::path::PathBuf;
use std::process;

use gruenes_gewolbe_core::{ManualFallbackCapture, Vault};

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(output) => {
            println!("{output}");
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let [command, rest @ ..] = args.as_slice() else {
        return Err(usage());
    };

    match command.as_str() {
        "create" => {
            let [path] = rest else {
                return Err(usage());
            };
            let path = PathBuf::from(path);
            let vault = Vault::create(&path).map_err(|error| error.to_string())?;
            Ok(format!("created\t{}", vault.root().display()))
        }
        "open" => {
            let [path] = rest else {
                return Err(usage());
            };
            let path = PathBuf::from(path);
            let vault = Vault::open(&path).map_err(|error| error.to_string())?;
            Ok(format!("opened\t{}", vault.root().display()))
        }
        "validate" => {
            let [path] = rest else {
                return Err(usage());
            };
            let path = PathBuf::from(path);
            Vault::validate(&path).map_err(|error| error.to_string())?;
            Ok(format!("valid\t{}", path.display()))
        }
        "import-paintings" => {
            let [vault_path, source_folder] = rest else {
                return Err(usage());
            };
            let vault_path = PathBuf::from(vault_path);
            let source_folder = PathBuf::from(source_folder);
            let vault = Vault::open(&vault_path).map_err(|error| error.to_string())?;
            let imported = vault
                .import_paintings_folder(source_folder)
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "imported-paintings\t{}\t{}",
                vault.root().display(),
                imported.len()
            ))
        }
        "add-files" => {
            let [vault_path, source_files @ ..] = rest else {
                return Err(usage());
            };
            if source_files.is_empty() {
                return Err(usage());
            }
            let vault =
                Vault::open(PathBuf::from(vault_path)).map_err(|error| error.to_string())?;
            let added = vault
                .add_artwork_files(source_files.iter().map(PathBuf::from))
                .map_err(|error| error.to_string())?;
            Ok(added
                .into_iter()
                .map(|item| {
                    format!(
                        "added-artwork\t{}\t{}\t{}",
                        item.id(),
                        cli_field(item.home_subvault()),
                        cli_field(&item.item_folder().display().to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "rebuild-index" => {
            let [vault_path] = rest else {
                return Err(usage());
            };
            let vault_path = PathBuf::from(vault_path);
            let vault = Vault::open(&vault_path).map_err(|error| error.to_string())?;
            let rebuilt = vault
                .rebuild_metadata_index()
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "rebuilt-metadata-index\t{}\t{}",
                vault.root().display(),
                rebuilt.indexed_items()
            ))
        }
        "search" => {
            let [vault_path, query] = rest else {
                return Err(usage());
            };
            let vault_path = PathBuf::from(vault_path);
            let vault = Vault::open(&vault_path).map_err(|error| error.to_string())?;
            let results = vault
                .search_metadata(query)
                .map_err(|error| error.to_string())?;
            Ok(results
                .into_iter()
                .map(|result| {
                    let saved_item = result.saved_item();
                    format!(
                        "search-result\t{}\t{}\t{}",
                        saved_item.id(),
                        saved_item.home_subvault(),
                        saved_item.item_folder().display()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "capture-manual-text" => {
            let [vault_path, source_link, title, saving_reason, copied_text] = rest else {
                return Err(usage());
            };
            let vault_path = PathBuf::from(vault_path);
            let vault = Vault::open(&vault_path).map_err(|error| error.to_string())?;
            let captured = vault
                .manual_fallback_capture(ManualFallbackCapture {
                    source_link: source_link.to_string(),
                    title: title.to_string(),
                    saving_reason: Some(saving_reason.to_string()),
                    copied_text: Some(copied_text.to_string()),
                    copied_image: None,
                })
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "captured-manual-text\t{}\t{}\t{}",
                captured.id(),
                captured.home_subvault(),
                captured.item_folder().display()
            ))
        }
        "inspect-item" => {
            let [vault_path, item_id] = rest else {
                return Err(usage());
            };
            let vault_path = PathBuf::from(vault_path);
            let vault = Vault::open(&vault_path).map_err(|error| error.to_string())?;
            let details = match vault.item_details(item_id) {
                Ok(details) => details,
                Err(error) => {
                    let message = error.to_string();
                    vault
                        .record_error_event("inspect-item", item_id, &message)
                        .map_err(|error| error.to_string())?;
                    return Err(format!("error\tinspect-item\t{}", cli_field(&message)));
                }
            };
            let folder_name = details
                .item_folder()
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(format!(
                "item\t{}\t{}\t{}\t{}\nfolder\tname\t{}\nfolder\tpath\t{}\nfield\ttitle\t{}\nfield\tcreator\t{}\nfield\tyear\t{}\nfield\treview_status\t{}\nfile\tprimary\t{}",
                details.id(),
                cli_field(details.home_subvault()),
                cli_field(&details.item_folder().display().to_string()),
                cli_field(details.title()),
                cli_field(&folder_name),
                cli_field(&details.item_folder().display().to_string()),
                cli_field(details.title()),
                cli_field(details.creator()),
                cli_field(details.year()),
                cli_field(details.review_status()),
                cli_field(&details.primary_file().display().to_string()),
            ))
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: ggvault <create|open|validate|rebuild-index> <vault-path> | ggvault add-files <vault-path> <image-file>... | ggvault import-paintings <vault-path> <source-folder> | ggvault search <vault-path> <query> | ggvault capture-manual-text <vault-path> <source-link> <title> <saving-reason> <copied-text> | ggvault inspect-item <vault-path> <item-id>"
        .to_string()
}

fn cli_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}
