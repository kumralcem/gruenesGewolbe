Status: ready-for-agent

# Launch and Reopen a Format-Version-2 Vault

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the first real Linux desktop path for launching the Tauri workbench, creating a format-version-2 Vault through a native folder dialog, opening an existing Vault, and returning to the last Active Vault after restart. This slice replaces the static scaffold with an actual runtime while keeping user-specific known-Vault state outside canonical Vault files.

## Acceptance criteria

- [x] A repository launch script starts a real Linux Tauri window, and a separate build script produces a local release binary without packaging an installer.
- [x] The frontend uses modular TypeScript and a typed command client whose payloads are handled by registered Tauri commands.
- [x] A user can create a format-version-2 Vault by selecting a new or empty folder through a native dialog.
- [x] Creation refuses an unrelated non-empty folder without writing Vault files into it.
- [x] A user can open an existing valid format-version-2 Vault through a native dialog.
- [x] Previously opened Vaults and the last Active Vault are remembered in user-specific app state rather than canonical Vault files.
- [x] Launch automatically reopens the last valid Active Vault; a missing or invalid last Vault returns the user to explicit create/open choices.
- [x] The workbench exposes only functional offline controls and does not show URL Capture or AI Enrichment placeholders.
- [x] Archive-core, Tauri-command, and browser-level tests cover create, refusal, open, remembered Vaults, restart, missing paths, and visible error states.

## Blocked by

None - can start immediately

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/vault_lifecycle.rs`, `apps/desktop/tests/active_vault.rs`, `apps/desktop/tests/tauri_commands.rs`, `apps/desktop/tests/tauri_scaffold.rs`, and `apps/desktop/tests/browser/startup.spec.ts`. `run.sh` launches the real Tauri 2 window, while `build.sh` produced `target/release/gruenes-gewolbe` with installer bundling disabled. The app uses native folder dialogs, stores known and last-active Vault paths under the user-specific Tauri app-data directory, restores a valid last Vault, and exposes missing or invalid last-Vault state as a visible notice with create/open recovery actions.
