Status: ready-for-agent

# URL and Manual Fallback Capture

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first capture workflow for saving a source link with an optional saving reason. The workflow should use best-effort extraction when possible and provide manual fallback when extraction is blocked or incomplete. Visual captures should preserve the best available file from the chosen source. Idea captures should preserve cleaned text when possible.

Manual fallback should accept a source link, saving reason, copied image data, and copied text while keeping provenance clear. Surrounding discussion should be excluded by default.

## Acceptance criteria

- [x] A user can save a source link with an optional saving reason into the active vault.
- [x] A visual capture preserves the best available file from the chosen source without silently replacing it later.
- [x] An idea capture preserves cleaned text as the source copy when extraction succeeds.
- [x] Failed or incomplete extraction offers manual fallback instead of blocking capture.
- [x] Manual fallback records the source link, saving reason, copied image data and/or copied text, and review status.
- [x] Surrounding discussion is excluded by default unless the user saves it as a separate saved item.
- [x] Tests cover successful extraction, failed extraction, manual fallback records, source-link provenance, and cleaned text preservation.

## Blocked by

- .scratch/personal-archive-vault/issues/02-save-first-artwork-item.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/capture.rs`, `apps/desktop/tests/capture.rs`, `crates/archive-core/tests/workbench.rs`, and `crates/archive-cli/tests/capture.rs`. URL capture now goes through a source-extraction boundary and returns a prefilled manual fallback prompt when extraction is blocked rather than creating a partial item or failing hard.
