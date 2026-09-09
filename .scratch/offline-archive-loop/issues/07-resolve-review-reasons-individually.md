Status: ready-for-agent

# Resolve Review Reasons Individually

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Replace the prototype's manually clearable Review Status with individually resolvable Review Reasons. The Review Queue and editor should explain each concern, let the user accept, correct, or dismiss it, and derive Review Status automatically. Existing Metadata Suggestions must be reviewable without making live AI calls.

## Acceptance criteria

- [x] Format-version-2 Item Records persist distinct Review Reasons with the evidence needed to explain them.
- [x] Unknown or conflicting metadata, manual fallback state, preview failures, Duplicate Candidates, and Metadata Suggestions can appear as separate reasons on the same Saved Item.
- [x] The Review Queue lists Saved Items with unresolved reasons and opens the relevant item and reason.
- [x] A user can accept, correct, or dismiss each Review Reason independently.
- [x] Review Status remains `needs-review` while any reason is unresolved and becomes `reviewed` automatically only when none remain.
- [x] Metadata Suggestions support Accept, Edit then accept, and Dismiss without requiring a configured AI provider.
- [x] Accepted Metadata Suggestions retain compact Metadata Provenance.
- [x] No broad Mark Reviewed action can hide unresolved reasons.
- [x] Tests cover multiple simultaneous reasons, every resolution path, derived status transitions, Review Queue behavior, and provenance retention.

## Blocked by

- .scratch/offline-archive-loop/issues/04-run-recursive-paintings-import.md
- .scratch/offline-archive-loop/issues/06-edit-item-records-without-losing-file-changes.md

## Comments

Implemented typed, evidence-bearing Review Reasons in format-version-2 Item Records and made Review Status derive exclusively from unresolved reasons. Unknown metadata, Manual Fallback, Duplicate Candidates, Thumbnail Preview failures, and staged Metadata Suggestions now produce independently resolvable concerns. Archive-core exposes one optimistic resolution interface for Accept, Correct/Edit then accept, and Dismiss; Metadata Suggestion acceptance works offline, applies the selected value, removes only the matching suggestion, and retains compact Metadata Provenance. The desktop command adapter and workbench expose a reason-level Review Queue and reason cards with correction inputs and actions; the former broad Review Status update was removed. Evidence: `crates/archive-core/tests/review_reasons.rs`, AI/import/workbench integration tests, and `apps/desktop/tests/browser/review-reasons.spec.ts`. The complete Rust workspace and all 15 browser tests pass.
