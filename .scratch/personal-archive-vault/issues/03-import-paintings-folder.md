Status: ready-for-agent

# Import a Paintings Folder

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build a copy-by-default import workflow for an existing folder of artwork files into the Paintings subvault. Imported items should become artwork saved items with readable item folders, preserved files, item records, import provenance, and review status when metadata is incomplete or uncertain.

This slice should be demoable from both the app and the CLI, because Paintings import is one of the first agent- and script-friendly workflows.

## Acceptance criteria

- [x] A user can choose a local folder and import supported artwork files into the active vault.
- [x] Import copies files by default and leaves the source folder unchanged.
- [x] Each imported item records original filename, source folder, and import date as import provenance.
- [x] Folder names prefer creator, year, and title when known, with explicit unknown fallbacks.
- [x] Items with missing or uncertain metadata receive a review status that makes them findable later.
- [x] The CLI can run the same import workflow against an active or specified vault.
- [x] Tests cover copy-by-default behavior, provenance, readable folder creation, review status, and preservation of original file formats.

## Blocked by

- .scratch/personal-archive-vault/issues/02-save-first-artwork-item.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/paintings_import.rs`, `crates/archive-cli/tests/import_paintings.rs`, `crates/archive-cli/tests/inspect_item.rs`, and `apps/desktop/tests/import_paintings.rs`. Paintings import copies supported image files by default, preserves source folders, records original filename/source folder/import date provenance, uses readable known/unknown folder names, marks uncertain imports for review while keeping complete known imports out of the review queue, and is available through the CLI and desktop shell.
