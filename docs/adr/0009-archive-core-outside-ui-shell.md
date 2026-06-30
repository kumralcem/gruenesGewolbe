# Archive Core Outside UI Shell

Archive behavior should live in a Rust core library rather than directly inside the Tauri UI shell. The core owns item folders, item records, tag registry updates, indexing, and enrichment workflows so the same archive rules can later be used by a desktop app, CLI, tests, or agent scripts without driving the GUI.
