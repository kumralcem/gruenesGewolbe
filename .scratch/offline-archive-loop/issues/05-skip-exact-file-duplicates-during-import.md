Status: ready-for-agent

# Skip Exact File Duplicates During Import

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Make repeated Import Runs low-maintenance by recognizing exact content already preserved in the Vault. Exact File Duplicates should be skipped before another Item Folder is created, reported distinctly, and remain importable only through a deliberate override; uncertain overlap stays a Duplicate Candidate rather than being treated as exact.

## Acceptance criteria

- [ ] Selected-file and folder import check a content fingerprint against Preserved Files before copying the incoming file.
- [ ] An Exact File Duplicate is skipped by default without creating another Item Folder or Review Reason.
- [ ] Exact duplicates have their own count and file details in Import Run summaries.
- [ ] The user can explicitly import an Exact File Duplicate anyway, creating a separate Saved Item through the normal preservation path.
- [ ] Matching filenames, source paths, or descriptive metadata without exact content identity remain nonblocking Duplicate Candidates.
- [ ] Re-importing the same folder does not flood the Vault or Review Queue with byte-identical Saved Items.
- [ ] Tests cover exact matches across different filenames and source paths, default skipping, override, ambiguous matches, and summary reporting.

## Blocked by

- .scratch/offline-archive-loop/issues/04-run-recursive-paintings-import.md

