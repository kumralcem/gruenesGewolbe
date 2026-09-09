Status: ready-for-agent

# Rebuildable Metadata Search

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first derived index and metadata search workflow. The index should be rebuildable from canonical vault files, and search should find saved items by item record fields, tags, saving reason, summary, source link, import provenance, and collection references where present.

This slice should provide search through the archive core, expose it through the CLI, and add a basic app search surface for the active vault.

## Acceptance criteria

- [x] A missing or stale derived index can be rebuilt from item records and collection files.
- [x] Metadata search returns saved items by title, creator, year, tags, saving reason, summary, source link, original filename, and collection references.
- [x] Search works without paid AI calls or network access.
- [x] The CLI can rebuild the index and run metadata searches.
- [x] The app can search within the active vault and open a matching saved item.
- [x] Tests verify rebuild behavior by deleting derived state and rebuilding from visible vault files.

## Blocked by

- .scratch/personal-archive-vault/issues/02-save-first-artwork-item.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/metadata_search.rs`, `crates/archive-cli/tests/search.rs`, and `apps/desktop/tests/search.rs`. Metadata index rebuilds from canonical item records and collection files, refreshes after newly saved artwork so stale indexes do not hide fresh records, finds source URLs and AI summaries, and keeps collection references searchable even when the item-record backreference is missing and derived state has been deleted.
