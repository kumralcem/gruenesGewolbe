Status: ready-for-agent

# Repair a Structurally Incomplete Vault

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build a safe recovery path for a recognizable format-version-2 Vault whose required structural directories are missing. Opening should present a repair proposal, allow the user to recreate only safe empty structure, and then continue into the workbench without rewriting malformed or existing canonical content.

## Acceptance criteria

- [x] Opening a Vault with a valid configuration but missing required structural directories produces a repair proposal instead of silently changing the Vault or reporting it as wholly unusable.
- [x] The repair screen lists each directory that would be created and requires explicit confirmation.
- [x] Confirmed repair creates only missing safe empty directories and then opens the repaired Vault.
- [x] Repair never overwrites existing files or directories and never rewrites malformed Item Records.
- [x] Invalid configuration and unsafe structural conflicts remain clear errors rather than being treated as repairable automatically.
- [x] Cancelling repair leaves the selected folder unchanged.
- [x] Archive-core, Tauri-command, and browser-level tests cover repairable, cancelled, non-repairable, and successful-open outcomes.

## Blocked by

- .scratch/offline-archive-loop/issues/01-launch-and-reopen-format-v2-vault.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/vault_lifecycle.rs`, `apps/desktop/tests/active_vault.rs`, `apps/desktop/tests/tauri_commands.rs`, and `apps/desktop/tests/browser/startup.spec.ts`. Opening a recognizable incomplete format-version-2 Vault now returns an exact repair proposal without mutation; explicit confirmation recreates only missing safe directories and opens the Vault, while cancellation is non-mutating. TOML parsing rejects malformed configuration, structural file conflicts remain errors, and malformed Item Records are preserved byte-for-byte.
