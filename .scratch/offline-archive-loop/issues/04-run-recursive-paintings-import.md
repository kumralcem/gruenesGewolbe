Status: ready-for-agent

# Run a Recursive Paintings Import

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build a best-effort Import Run that recursively discovers ordinary image files, preserves independent successes, reports progress, supports cancellation, and ends with an auditable summary. Imported Artwork Saved Items should immediately use the same Thumbnail Preview, metadata, review, search, and Activity Log behavior as selected-file additions.

## Acceptance criteria

- [ ] A user can select a source folder through a native dialog and recursively import supported images from ordinary nested directories.
- [ ] Symbolic files and directories are not followed, and symbolic links plus unsupported files appear as skipped entries in the final summary.
- [ ] The workbench shows ongoing progress without freezing and allows cancellation between files.
- [ ] Cancellation keeps completed Saved Items and reports the Import Run as partially completed rather than rolling it back.
- [ ] An inaccessible, invalid, or failed source file does not roll back other Saved Items and appears in the summary and Activity Log.
- [ ] The final summary distinguishes imported, skipped, duplicate-candidate, cancelled, and failed files.
- [ ] Ambiguous provenance or descriptive overlap remains nonblocking, creates a Duplicate Candidate and Review Reason, and does not require AI.
- [ ] Derived metadata search is refreshed once per completed or cancelled Import Run rather than after every individual file.
- [ ] Archive-core, Tauri-command, and browser-level tests cover recursion, symlinks, progress, cancellation, partial failure, duplicate candidates, summary counts, Activity Log events, and batched indexing.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-artwork-saved-item.md
