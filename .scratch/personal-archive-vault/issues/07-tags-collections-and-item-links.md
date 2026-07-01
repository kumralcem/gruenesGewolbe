Status: ready-for-agent

# Tags, Collections, and Item Links

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first cross-item organization workflow. Tags should be normalized through a vault-level tag registry, collections should be durable files with item backreferences, and item links should express relevance across saved items, collections, notes, URLs, or archive areas without moving item folders or creating multiple home subvaults.

This slice should make cross-subvault relevance visible in the app and searchable through the derived index.

## Acceptance criteria

- [ ] A user can add tags to a saved item and reuse existing registry entries or aliases.
- [ ] A user can create genuinely new tags when no existing tag matches.
- [ ] A user can create a collection and add saved items to it without moving item folders.
- [ ] Collection membership is visible from the saved item detail view.
- [ ] A user can add item links to related saved items, collections, notes, URLs, or archive areas.
- [ ] Search can find saved items by tags, collection membership, and item-link text where appropriate.
- [ ] Tests verify tag registry normalization, collection files with item backreferences, and one-home-subvault behavior.

## Blocked by

- .scratch/personal-archive-vault/issues/04-rebuildable-metadata-search.md
- .scratch/personal-archive-vault/issues/05-workbench-browse-edit-review.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/organization.rs` and `apps/desktop/tests/organization.rs`. Tags normalize through the vault tag registry, collections are durable files with item backreferences, item details can recover collection membership from canonical collection files, item links do not move home subvault folders, and tags/collections/item links are searchable through rebuildable metadata search.
