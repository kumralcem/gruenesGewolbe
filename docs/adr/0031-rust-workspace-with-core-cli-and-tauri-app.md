# Rust Workspace With Core, CLI, and Tauri App

The project should start as a Rust workspace with an archive core crate, a minimal CLI crate, and a Tauri desktop app rather than growing all logic inside `src-tauri` first. This preserves the chosen boundaries: archive rules stay testable and reusable, agents can use CLI/core behavior without driving the GUI, and the Tauri app remains a shell around the portable vault workflows.
