Status: ready-for-agent

# Create and Open an Active Vault

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first end-to-end path for creating a vault as an ordinary local folder, opening an existing vault, and making it the active vault for app operations. The behavior should establish the vault as the source of truth, create the minimal visible vault structure, keep derived state rebuildable, and avoid requiring a background service.

This slice should include archive core behavior, a minimal CLI path for validation/opening, and a Tauri app path that lets the user create or open a vault and see that it is active.

## Acceptance criteria

- [x] A user can create a new vault in a local folder and reopen it later.
- [x] A user can open an existing vault copied from another location without hidden app state being required.
- [x] The active vault is explicit in the desktop app before capture, import, search, or browse operations run.
- [x] The vault contains only durable visible files for archive meaning; any hidden derived state can be deleted and rebuilt.
- [x] The CLI can validate whether a folder is a usable vault and report clear errors for invalid folders.
- [x] Automated tests create temporary vaults and assert on externally visible vault behavior, not private implementation details.

## Blocked by

None - can start immediately

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/vault_lifecycle.rs`, `apps/desktop/tests/active_vault.rs`, and `crates/archive-cli/tests/validate.rs`. The desktop shell now remembers multiple vault roots in user app state and can switch the active vault explicitly.
