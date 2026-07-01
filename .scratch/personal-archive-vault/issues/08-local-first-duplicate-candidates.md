Status: ready-for-agent

# Local-First Duplicate Candidate Warnings

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build duplicate candidate detection for import and capture workflows using local, cheap signals first. The app should warn when an incoming capture or import appears to overlap with an existing saved item, but it should not block saving automatically.

Duplicate signals should include source links, file fingerprints, import provenance, and descriptive metadata. Paid AI should not be required for normal duplicate candidate warnings.

## Acceptance criteria

- [ ] Import warns when an incoming file appears to match an existing saved item by fingerprint or provenance.
- [ ] URL capture warns when a source link already exists in the vault.
- [ ] Capture and import can warn on strong descriptive metadata overlap without requiring AI.
- [ ] Duplicate candidates are represented as review-worthy state rather than hard errors.
- [ ] The user can continue saving despite a duplicate candidate warning.
- [ ] Tests cover exact source-link matches, file fingerprint matches, provenance matches, metadata matches, and nonblocking behavior.

## Blocked by

- .scratch/personal-archive-vault/issues/03-import-paintings-folder.md
- .scratch/personal-archive-vault/issues/06-url-and-manual-fallback-capture.md
