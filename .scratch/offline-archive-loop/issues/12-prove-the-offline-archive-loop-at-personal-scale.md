Status: ready-for-agent

# Prove the Offline Archive Loop at Personal Scale

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Complete and verify the milestone as one uninterrupted personal workflow rather than a collection of individually passing adapters. Exercise the real Linux window, browser-level workbench behavior, restart and recovery, offline operation, and a generated personal-scale Vault, then align user-facing documentation with what can actually be launched and demonstrated.

## Acceptance criteria

- [ ] The documented Linux launch script opens the real workbench and the build script produces a runnable local release binary.
- [ ] A manual smoke run creates a Vault, adds selected files, imports nested paintings, shows actual previews, searches, edits, resolves review, uses Vault Trash, restores an item, closes, relaunches, and reopens the same canonical state.
- [ ] The smoke run includes an Exact File Duplicate, ambiguous Duplicate Candidate, preview failure, malformed record, cancelled Import Run, and recoverable repair path.
- [ ] Browser-level Playwright tests cover the complete workbench through controlled typed command responses, including loading, empty, progress, conflict, error, Problems, review, and Vault Trash states.
- [ ] Playwright screenshots at desktop and constrained viewports show rendered media, readable text, stable controls, and no incoherent overlap.
- [ ] Generated-data verification covers correct create/open, browse, metadata search, and derived-index rebuild behavior around 5,000 Saved Items and records measured timings without imposing unsupported optimization thresholds.
- [ ] All automated tests run without network access, OpenAI credentials, or live external services.
- [ ] Automated native-window WebDriver coverage remains explicitly deferred and documented for later reconsideration.
- [ ] User-facing documentation accurately distinguishes implemented offline workflows from deferred URL, AI, packaging, and cross-platform work.

## Blocked by

- .scratch/offline-archive-loop/issues/02-repair-structurally-incomplete-vault.md
- .scratch/offline-archive-loop/issues/04-run-recursive-paintings-import.md
- .scratch/offline-archive-loop/issues/05-skip-exact-file-duplicates-during-import.md
- .scratch/offline-archive-loop/issues/06-edit-item-records-without-losing-file-changes.md
- .scratch/offline-archive-loop/issues/07-resolve-review-reasons-individually.md
- .scratch/offline-archive-loop/issues/08-keep-vault-usable-around-malformed-records.md
- .scratch/offline-archive-loop/issues/09-move-items-to-vault-trash-and-restore.md
- .scratch/offline-archive-loop/issues/10-resolve-duplicate-candidates-without-merging.md
- .scratch/offline-archive-loop/issues/11-permanently-delete-a-trashed-item.md
