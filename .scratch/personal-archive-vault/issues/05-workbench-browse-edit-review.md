Status: ready-for-agent

# Workbench Browse, Edit, and Review

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first useful workbench experience for browsing saved items, inspecting item details, editing item records, and clearing review status. The workbench should support visual scanning for artwork saved items and a review-focused view for uncertain imports or captures.

The app should remain a shell around archive core behavior: direct file edits to item records should remain valid, and the app should refresh derived state from canonical vault files.

## Acceptance criteria

- [x] A user can browse artwork saved items in a dense visual grid.
- [x] A user can open an item detail view and inspect preserved file, item record fields, source link, provenance, tags, and review status.
- [x] A user can edit item record fields through the UI and see those edits persisted to the vault files.
- [x] A user can clear or update review status from the UI.
- [x] Direct edits to item records on disk are reflected after refresh or index rebuild.
- [x] UI tests cover opening a vault, browsing the Paintings subvault, searching, inspecting an item, editing metadata, and updating review status.

## Blocked by

- .scratch/personal-archive-vault/issues/03-import-paintings-folder.md
- .scratch/personal-archive-vault/issues/04-rebuildable-metadata-search.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/workbench.rs`, `apps/desktop/tests/workbench.rs`, `apps/desktop/tests/capture.rs`, and `apps/desktop/tests/import_paintings.rs`. The workbench path includes artwork browsing with rebuildable cached thumbnail previews, Idea Sources list/detail browsing with source links, item details, record updates, review status updates, direct file edit refresh, a cross-subvault review queue for `needs-review` items, and folder rename suggestions after metadata cleanup without silently moving item folders.
