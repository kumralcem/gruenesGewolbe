Status: ready-for-agent

# URL and Manual Fallback Capture

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first capture workflow for saving a source link with an optional saving reason. The workflow should use best-effort extraction when possible and provide manual fallback when extraction is blocked or incomplete. Visual captures should preserve the best available file from the chosen source. Idea captures should preserve cleaned text when possible.

Manual fallback should accept a source link, saving reason, copied image data, and copied text while keeping provenance clear. Surrounding discussion should be excluded by default.

## Acceptance criteria

- [x] A user can save a source link with an optional saving reason from the native workbench into the Active Vault.
- [x] A native visual capture preserves the Best Available File from the chosen source without silently replacing it later.
- [x] An idea capture preserves cleaned text as the source copy when extraction succeeds.
- [x] Failed or incomplete extraction offers manual fallback instead of blocking capture.
- [x] Manual fallback records the source link, saving reason, copied image data and/or copied text, and review status.
- [x] Surrounding discussion is excluded by default unless the user saves it as a separate saved item.
- [x] Tests cover successful extraction, failed extraction, manual fallback records, source-link provenance, and cleaned text preservation.

## Blocked by

- .scratch/personal-archive-vault/issues/02-save-first-artwork-item.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/capture.rs`, `apps/desktop/tests/capture.rs`, `crates/archive-core/tests/workbench.rs`, and `crates/archive-cli/tests/capture.rs`. URL capture now goes through a source-extraction boundary: successful extraction persists cleaned text into the vault, while blocked extraction returns a prefilled manual fallback prompt rather than creating a partial item or failing hard.

- 2026-08-20 dogfood correction: the checked evidence above proves archive-core, desktop-shell, and CLI boundaries with fake extractors, but the Tauri command is not registered and the workbench exposes neither URL Capture nor Manual Fallback. The remaining agent-ready slice must add the native capture surface and real bounded extractor adapters (with X.com fallback and Wikimedia handling); it must not present manual URL storage as automatic extraction.

- 2026-08-29 native capture slice: Capture Link is on the workbench. Wikimedia Commons pages and `upload.wikimedia.org` media persist as Paintings artwork with Source Link provenance and `capture_method: extracted-image`. X.com, Wikipedia article hosts, generic pages, non-HTTPS, and credentialed URLs return a prefilled Manual Fallback into Idea Sources. Copied text and pasted image bytes keep the Source Link. Surrounding discussion is not fetched. Network extraction runs outside the desktop state mutex; persistence is rejected if the Active Vault changes. Wikimedia records omit Import Provenance and never point at `.gruenesgewolbe/capture-staging`. Local imports still omit `source_link` / `capture_method`. Bounded `reqwest` (optional, rustls, blocking, 15s timeout, 2 MiB page / 50 MiB image, max 4 Wikimedia-only redirects) is the HTTPS adapter; generic crawling remains out of this slice. Native GUI smoke of one Wikimedia URL and one X/manual-fallback URL is still requested separately and was not run here.

  Verification (serial; Steam closed; Playwright `--workers=2`; no native window):
  - `cargo test -p gruenes-gewolbe-core --test capture --test artwork_items --test paintings_import`
  - `cargo test -p gruenes-gewolbe-desktop --test tauri_commands --test capture --test tauri_scaffold`
  - `cargo test -p gruenes-gewolbe-desktop --all-features source_extractor` (7/7)
  - `cargo check -p gruenes-gewolbe-desktop --all-features`
  - `pnpm build` in `apps/desktop`
  - `pnpm exec playwright test --workers=2 tests/browser/url-capture.spec.ts` (2/2)
  - `cargo test -p gruenes-gewolbe-core`
  - `cargo test -p gruenes-gewolbe-desktop`
  - `cargo test -p gruenes-gewolbe-cli`
  - `pnpm exec playwright test --workers=2` in `apps/desktop` (30/30 after layout snapshot update)

  Review vs `c7431cc`: Manual Fallback title is optional (Untitled Capture). Wikimedia extraction prefers the original upload over `/thumb/` and unwraps Commons thumbnail Open Graph URLs. Capture draft reset is centralized. Active Vault change is enforced once at persist time. `imported_at` remains on captured artwork because gallery newest-sort already reads it as added_at; Import Provenance path/filename fields stay omitted. Generic cleaned-text extraction and native GUI smoke were not added.

- 2026-08-29 dogfood: three native captures (two X posts and Commons `File:The_Four_Horsemen_(CBL_WEp_0021).jpg`) all landed as empty Manual Fallback Idea Sources. The Commons original is 50.83 MiB, over the 50 MiB capture limit, and the extractor stopped instead of taking the 3840px preview. X returned fallback before public unfurl, and the workbench required a second Manual Fallback submit. Capture now HEADs candidates and keeps the largest in-budget Wikimedia file; public X `og:image` media on `pbs.twimg.com/media/` is preserved as Paintings; unsupported links auto-save as Idea Sources from the Source Link field (paste submits). Still no logged-in X harvesting or unofficial proxies.

- 2026-09-08 implementation: Idea Sources is now a separate navigation area with URL/pasted-text capture, bounded main-content extraction, local source reading, and app-generated summaries when OpenAI is configured. Empty fallback no longer silently saves a bookmark. Final automated validation passed (163 Rust tests, 36 browser tests, frontend build); see `.scratch/complete-idea-archive/VALIDATION.md`. Native-window/live-source acceptance and paid-provider verification have not been claimed.
