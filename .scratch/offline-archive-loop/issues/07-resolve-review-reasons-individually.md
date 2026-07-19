Status: ready-for-agent

# Resolve Review Reasons Individually

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Replace the prototype's manually clearable Review Status with individually resolvable Review Reasons. The Review Queue and editor should explain each concern, let the user accept, correct, or dismiss it, and derive Review Status automatically. Existing Metadata Suggestions must be reviewable without making live AI calls.

## Acceptance criteria

- [ ] Format-version-2 Item Records persist distinct Review Reasons with the evidence needed to explain them.
- [ ] Unknown or conflicting metadata, manual fallback state, preview failures, Duplicate Candidates, and Metadata Suggestions can appear as separate reasons on the same Saved Item.
- [ ] The Review Queue lists Saved Items with unresolved reasons and opens the relevant item and reason.
- [ ] A user can accept, correct, or dismiss each Review Reason independently.
- [ ] Review Status remains `needs-review` while any reason is unresolved and becomes `reviewed` automatically only when none remain.
- [ ] Metadata Suggestions support Accept, Edit then accept, and Dismiss without requiring a configured AI provider.
- [ ] Accepted Metadata Suggestions retain compact Metadata Provenance.
- [ ] No broad Mark Reviewed action can hide unresolved reasons.
- [ ] Tests cover multiple simultaneous reasons, every resolution path, derived status transitions, Review Queue behavior, and provenance retention.

## Blocked by

- .scratch/offline-archive-loop/issues/04-run-recursive-paintings-import.md
- .scratch/offline-archive-loop/issues/06-edit-item-records-without-losing-file-changes.md

