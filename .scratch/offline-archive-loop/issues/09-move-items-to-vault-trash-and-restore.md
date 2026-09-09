Status: ready-for-human

# Move Items to Vault Trash and Restore Them

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build recoverable removal through visible canonical Vault Trash. A user or agent should be able to move an Item Folder out of active browsing, inspect its trashed state through retained references, and restore it to its Home Subvault without overwriting an occupied path.

## Acceptance criteria

- [x] Moving a Saved Item to Vault Trash relocates its complete Item Folder under `trash/<home-subvault>/` without deleting canonical content.
- [x] Trashed items are excluded from normal gallery browsing, search results, collection counts, and the Review Queue.
- [x] Collection membership and incoming Item Links remain canonical and visibly identify that their target is in Vault Trash.
- [x] The workbench provides a Vault Trash view with item details and a restore action.
- [x] Restore returns the item to its Home Subvault and preserves its stable item ID and retained references.
- [x] Restore never overwrites an occupied Item Folder and uses a small unique suffix when necessary.
- [x] Vault Trash has no automatic purge behavior.
- [x] Structured CLI commands can move and restore an explicitly identified item.
- [x] Tests cover move, active-view exclusion, retained references, restore, path collision, restart, derived-index rebuild, and CLI behavior.

## Blocked by

- .scratch/offline-archive-loop/issues/08-keep-vault-usable-around-malformed-records.md

## Implementation notes

Implemented visible canonical Vault Trash across archive-core, CLI, typed desktop commands, and the browser workbench. Existing active Item Record scanning already excluded root-level Trash from gallery, search-derived state, collection counts, and Review Queue; this slice adds collision-safe relocation and restoration, retained Collection and incoming Item Link context, Activity Log entries, and localized Trash scanning that leaves malformed Item Records untouched.

Verification evidence: `crates/archive-core/tests/vault_trash.rs`, `crates/archive-cli/tests/vault_trash.rs`, and `apps/desktop/tests/browser/vault-trash.spec.ts`; `cargo test --workspace --all-features` passed; `pnpm --dir apps/desktop typecheck` and `pnpm --dir apps/desktop build` passed; all 17 Playwright tests passed; `git diff --check` passed.
