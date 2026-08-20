Status: ready-for-human

# Prove the Offline Archive Loop at Personal Scale

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Complete and verify the milestone as one uninterrupted personal workflow rather than a collection of individually passing adapters. Exercise the real Linux window, browser-level workbench behavior, restart and recovery, offline operation, and a generated personal-scale Vault, then align user-facing documentation with what can actually be launched and demonstrated.

## Acceptance criteria

- [x] The documented Linux launch script opens the real workbench and the build script produces a runnable local release binary.
- [ ] A manual smoke run creates a Vault, adds selected files, imports nested image files into the Paintings Subvault, shows actual previews, searches, edits, resolves review, uses Vault Trash, restores an item, closes, relaunches, and reopens the same canonical state.
- [ ] The smoke run includes an Exact File Duplicate, ambiguous Duplicate Candidate, preview failure, malformed record, cancelled Import Run, and recoverable repair path.
- [x] Browser-level Playwright tests cover the complete workbench through controlled typed command responses, including loading, empty, progress, conflict, error, Problems, review, and Vault Trash states.
- [x] Playwright screenshots at desktop and constrained viewports show rendered media, readable text, stable controls, and no incoherent overlap.
- [x] Generated-data verification covers correct create/open, browse, metadata search, and derived-index rebuild behavior around 5,000 Saved Items and records measured timings without imposing unsupported optimization thresholds.
- [x] All automated tests run without network access, OpenAI credentials, or live external services.
- [x] Automated native-window WebDriver coverage remains explicitly deferred and documented for later reconsideration.
- [x] User-facing documentation accurately distinguishes implemented offline workflows from deferred URL, AI, packaging, and cross-platform work.

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

## Comments

- 2026-08-20: Existing focused coverage already satisfied the typed-command/browser behavior for first launch, empty and error states, repair, selected files, recursive import, progress/cancellation, Exact File Duplicates, Duplicate Candidates, preview failure, search, Item Record conflicts, reason-level review, localized malformed records per ADR-0036, Vault Trash, restore, permanent deletion, and restart-safe Active Vault state. The malformed-record implementation remains shared archive-core scanning surfaced through CLI, desktop snapshots, and the workbench; malformed canonical bytes are not rewritten.
- 2026-08-20: Fixed `run.sh` and `build.sh` for pnpm 10 by removing the argument separator that caused Tauri to reject its `dev`/`build` subcommands. `./build.sh` produced `/home/cem/Sync/Projects/gruenesGewolbe/target/release/gruenes-gewolbe`; `./run.sh` reached Vite readiness and launched `/home/cem/Sync/Projects/gruenesGewolbe/target/debug/gruenes-gewolbe` as a persistent Tauri process.
- 2026-08-20: Added generated 5,000-item public-API verification for create/open, gallery browse (including rebuilding ten uncached Thumbnail Previews), Derived Index rebuild, and metadata search. Focused timings on this machine were open 166.965 µs, browse 11.891 s, rebuild 3.743 s, and search 101.130 ms; these are recorded evidence, not pass/fail thresholds.
- 2026-08-20: Added retained Playwright screenshot baselines and overflow checks at 1440×1000 and 720×900 with rendered media, Vault Problems, Review Queue, search results, and Vault Trash. Both baselines under `apps/desktop/tests/browser/workbench-layout.spec.ts-snapshots/` were visually inspected for readable text, stable controls, and overlap. Verification passed in the network-restricted workspace without external credentials or services: `cargo test --workspace --all-features`; `pnpm build`; `pnpm exec playwright test` (23/23); focused personal-scale test with `--nocapture`; `git diff --check`; and the local release build.
- 2026-08-20: Added `docs/offline-archive-loop-smoke.md` with the exact native-window workflow, edge fixtures, recovery checks, and retained evidence. The two manual smoke criteria remain unchecked pending a human-driven native-dialog run; automated native-window WebDriver remains deliberately deferred by the PRD.
- 2026-08-20: Follow-up dogfooding exposed a synchronous preview-generation freeze after a successful 132-item import. The first uncached boundary file was an 8000×8000 WebP: the old gallery path took about 37 seconds on that file alone and the first 25 previews took 74 seconds. Gallery snapshots now create lightweight pending placeholders immediately, Tauri prepares one preview per background batch, and the workbench refreshes incrementally. A 24-megapixel safety limit rejects oversized sources before full decode while preserving the original and recording the normal Thumbnail Preview Review Reason. Replay of the exact reported WebP reduced initial browse to 131 ms and the bounded placeholder decision to 136 ms. Verification: full Rust workspace passed; production TypeScript/Vite build passed; Playwright passed 24/24 including the new background-preparation test.
