Status: ready-for-agent

# Skip Exact File Duplicates During Import

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Make repeated Import Runs low-maintenance by recognizing exact content already preserved in the Vault. Exact File Duplicates should be skipped before another Item Folder is created, reported distinctly, and remain importable only through a deliberate override; uncertain overlap stays a Duplicate Candidate rather than being treated as exact.

## Acceptance criteria

- [x] Selected-file and folder import check a content fingerprint against Preserved Files before copying the incoming file.
- [x] An Exact File Duplicate is skipped by default without creating another Item Folder or Review Reason.
- [x] Exact duplicates have their own count and file details in Import Run summaries.
- [x] The user can explicitly import an Exact File Duplicate anyway, creating a separate Saved Item through the normal preservation path.
- [x] Matching filenames, source paths, or descriptive metadata without exact content identity remain nonblocking Duplicate Candidates.
- [x] Re-importing the same folder does not flood the Vault or Review Queue with byte-identical Saved Items.
- [x] Tests cover exact matches across different filenames and source paths, default skipping, override, ambiguous matches, and summary reporting.

## Blocked by

- .scratch/offline-archive-loop/issues/04-run-recursive-paintings-import.md

## Comments

Implemented byte-verified Exact File Duplicate handling for both selected files and folder Import Runs. Default imports skip before Item Folder creation and report the existing item; deliberate override preserves a separate item without creating exact-identity review noise, including when several approved exact copies already exist. Filename, provenance, and descriptive overlap with different bytes remain Duplicate Candidates. Selected-file outcomes are named separately from folder Import Runs in APIs, UI, and Activity Log events. Coverage includes renamed exact copies, durable Preserved File lookup, overrides, repeated folder imports, multiple approved copies, filename-only ambiguity, unsupported selected files, Tauri serialization, and browser summary rendering.
