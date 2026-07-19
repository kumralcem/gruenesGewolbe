Status: ready-for-agent

# Keep the Vault Usable Around Malformed Records

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the resilient browsing and search path for a file-first Vault where one canonical Item Record may be malformed. The workbench should keep valid Saved Items usable, report localized Vault Problems with actionable context, skip only affected records during derived rebuilds, and make search results and existing organization context navigable.

## Acceptance criteria

- [ ] Opening a Vault succeeds when one Item Record is malformed and other valid Saved Items remain browseable.
- [ ] The malformed file is left untouched and appears as a Vault Problem with its path and parse error.
- [ ] Metadata index rebuild and search skip only malformed records, continue with valid records, and report omitted paths.
- [ ] Correcting the malformed file externally and refreshing removes the Vault Problem and returns the item to browsing and search.
- [ ] Search works across the Active Vault without network or AI calls, and selecting a result opens Item Details.
- [ ] Item Details shows existing Collection membership and Item Links without adding collection or link editing controls.
- [ ] Contextual failures and Import Run summaries are visible in the workbench, with a command to open the underlying Activity Log for deeper inspection.
- [ ] The CLI lists Vault Problems in structured output suitable for agents and scripts.
- [ ] Tests cover multiple valid and malformed records, omission reporting, external correction, search navigation, organization details, Activity Log access, and CLI output.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-painting.md

