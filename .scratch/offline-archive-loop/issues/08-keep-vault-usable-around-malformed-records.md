Status: ready-for-human

# Keep the Vault Usable Around Malformed Records

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the resilient browsing and search path for a file-first Vault where one canonical Item Record may be malformed. The workbench should keep valid Saved Items usable, report localized Vault Problems with actionable context, skip only affected records during derived rebuilds, and make search results and existing organization context navigable.

## Acceptance criteria

- [x] Opening a Vault succeeds when one Item Record is malformed and other valid Saved Items remain browseable.
- [x] The malformed file is left untouched and appears as a Vault Problem with its path and parse error.
- [x] Metadata index rebuild and search skip only malformed records, continue with valid records, and report omitted paths.
- [x] Correcting the malformed file externally and refreshing removes the Vault Problem and returns the item to browsing and search.
- [x] Search works across the Active Vault without network or AI calls, and selecting a result opens Item Details.
- [x] Item Details shows existing Collection membership and Item Links without adding collection or link editing controls.
- [x] Contextual failures and Import Run summaries are visible in the workbench, with a command to open the underlying Activity Log for deeper inspection.
- [x] The CLI lists Vault Problems in structured output suitable for agents and scripts.
- [x] Tests cover multiple valid and malformed records, omission reporting, external correction, search navigation, organization details, Activity Log access, and CLI output.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-artwork-saved-item.md

## Comments

Implemented ADR-0036 localization through a shared archive-core Item Record scan: malformed YAML, missing required fields, unreadable records, and unsupported item types become path-and-error `VaultProblem` values while browsing, the Review Queue, and metadata rebuild continue with valid Saved Items. Rebuild results expose omitted paths; explicit workbench refresh rebuilds search so externally corrected records return to browsing/search and their Vault Problem disappears. Search results now include display titles and open Item Details, whose existing read-only Collection membership and Item Links remain visible.

The desktop command contract and browser workbench surface Vault Problems, offline search, refresh, contextual errors, existing Import Run summaries, and an `Open Activity Log` action backed by Tauri's opener plugin. `ggvault problems <vault-path>` emits escaped TSV suitable for scripts, while `rebuild-index` emits one `omitted-item-record` line per skipped path.

Verification evidence: `crates/archive-core/tests/malformed_records.rs`, `crates/archive-cli/tests/problems.rs`, `apps/desktop/tests/malformed_records.rs`, and `apps/desktop/tests/browser/malformed-records.spec.ts`; `cargo test --workspace` passed; `pnpm --dir apps/desktop typecheck` and `pnpm --dir apps/desktop build` passed; all 16 Playwright tests passed with `pnpm exec playwright test`. `git diff --check` passed. Rust formatting could not be run because this environment does not have `cargo fmt`/`rustfmt` installed.
