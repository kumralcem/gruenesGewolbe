Status: ready-for-human

# Resolve Duplicate Candidates Without Merging

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Complete the ambiguous duplicate review path without introducing record merging. From the Duplicate Candidate Review Reason, the user should be able to declare that the items are unrelated, retain both intentionally, or recoverably remove the current item through Vault Trash.

## Acceptance criteria

- [x] A Duplicate Candidate Review Reason shows the candidate item, matching evidence, and enough details to compare the Saved Items.
- [x] Not a Duplicate records the user's decision and resolves the reason without changing either Item Folder.
- [x] Keep Both records an intentional-overlap decision and resolves the reason without changing either Item Folder.
- [x] Move This Item to Vault Trash uses the standard recoverable-removal workflow and keeps the other Saved Item active.
- [x] Each resolution updates derived Review Status consistently and removes resolved work from the Review Queue.
- [x] Exact File Duplicate import skipping remains separate from ambiguous Duplicate Candidate resolution.
- [x] No merge-files or merge-records operation is introduced.
- [x] Tests cover all three actions, evidence display, Review Status transitions, restart, and interactions with Vault Trash.

## Blocked by

- .scratch/offline-archive-loop/issues/05-skip-exact-file-duplicates-during-import.md
- .scratch/offline-archive-loop/issues/07-resolve-review-reasons-individually.md
- .scratch/offline-archive-loop/issues/09-move-items-to-vault-trash-and-restore.md

## Comments

- 2026-08-20: Existing behavior already kept ambiguous Duplicate Candidates nonblocking, kept Exact File Duplicate skipping separate, derived Review Status from unresolved reasons, localized malformed Item Records for browse/index operations, and provided standard Vault Trash move/restore behavior.
- 2026-08-20: Added a dedicated duplicate-resolution contract across archive-core, structured CLI, Tauri commands, typed frontend adapter, and browser workbench. Decisions are written to the Item Record's `Duplicate Candidate Decisions` section; active decisions preserve both Item Folders, while the trash decision delegates to the standard recoverable-removal path. The resolver scans valid records so unrelated malformed Item Records remain localized per ADR-0036.
- 2026-08-20 verification: `cargo test --workspace` passed; `pnpm --dir apps/desktop build` passed TypeScript typechecking and the Vite production build; `pnpm --dir apps/desktop exec playwright test --reporter=line` passed all 20 browser tests (including three duplicate-resolution cases). `cargo fmt --all -- --check` remains red because the branch's pre-existing committed Rust sources are broadly unformatted; no unrelated formatting rewrite was included in this slice.
- 2026-08-20 review follow-up: Added explicitly serialized candidate identity and Idea Source comparison details, covered the non-terminal `needs-review` transition when another reason remains, and made the Vault Trash branch roll back the standard move if the canonical decision write fails.
