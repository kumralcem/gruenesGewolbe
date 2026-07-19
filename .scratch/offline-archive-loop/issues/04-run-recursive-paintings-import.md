Status: ready-for-agent

# Run a Recursive Paintings Import

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build a best-effort Import Run that recursively discovers ordinary image files, preserves independent successes, reports progress, supports cancellation, and ends with an auditable summary. Imported Artwork Saved Items should immediately use the same Thumbnail Preview, metadata, review, search, and Activity Log behavior as selected-file additions.

## Acceptance criteria

- [x] A user can select a source folder through a native dialog and recursively import supported images from ordinary nested directories.
- [x] Symbolic files and directories are not followed, and symbolic links plus unsupported files appear as skipped entries in the final summary.
- [x] The workbench shows ongoing progress without freezing and allows cancellation between files.
- [x] Cancellation keeps completed Saved Items and reports the Import Run as partially completed rather than rolling it back.
- [x] An inaccessible, invalid, or failed source file does not roll back other Saved Items and appears in the summary and Activity Log.
- [x] The final summary distinguishes imported, skipped, duplicate-candidate, cancelled, and failed files.
- [x] Ambiguous provenance or descriptive overlap remains nonblocking, creates a Duplicate Candidate and Review Reason, and does not require AI.
- [x] Derived metadata search is refreshed once per completed or cancelled Import Run rather than after every individual file.
- [x] Archive-core, Tauri-command, and browser-level tests cover recursion, symlinks, progress, cancellation, partial failure, duplicate candidates, summary counts, Activity Log events, and batched indexing.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-artwork-saved-item.md

## Comments

Implemented as a recursive, symlink-safe, best-effort Import Run across archive-core, asynchronous Tauri commands, and the browser workbench. The run reports typed per-file outcomes, preserves completed work through cancellation and source failures, localizes malformed vault records, batches derived-index refresh, and exposes progress, cancellation, optional metadata, exact-duplicate policy, and an auditable summary. Verification includes the full Rust workspace with all features, Tauri runtime compilation, the production frontend build, and all browser tests.
