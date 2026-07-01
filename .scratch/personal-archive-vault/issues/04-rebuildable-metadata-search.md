Status: ready-for-agent

# Rebuildable Metadata Search

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first derived index and metadata search workflow. The index should be rebuildable from canonical vault files, and search should find saved items by item record fields, tags, saving reason, summary, source link, import provenance, and collection references where present.

This slice should provide search through the archive core, expose it through the CLI, and add a basic app search surface for the active vault.

## Acceptance criteria

- [ ] A missing or stale derived index can be rebuilt from item records and collection files.
- [ ] Metadata search returns saved items by title, creator, year, tags, saving reason, summary, source link, original filename, and collection references.
- [ ] Search works without paid AI calls or network access.
- [ ] The CLI can rebuild the index and run metadata searches.
- [ ] The app can search within the active vault and open a matching saved item.
- [ ] Tests verify rebuild behavior by deleting derived state and rebuilding from visible vault files.

## Blocked by

- .scratch/personal-archive-vault/issues/02-save-first-artwork-item.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/metadata_search.rs`, `crates/archive-cli/tests/search.rs`, and `apps/desktop/tests/search.rs`. Metadata index rebuilds from canonical item records and collection files, so collection references remain searchable even when the item-record backreference is missing and derived state has been deleted.
