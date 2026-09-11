# Restart review — 2026-09-11

Fixed comparison: `git diff --find-renames 745f00dc2d986817c895fe83d8ec25b0d8fab4a3...f41a976`. Commit `f41a976` replaces the legacy app with the root Pi CLI, Chrome capture, and Obsidian records. Separate reviewers checked Standards and Spec using the repository review skill. Follow-up fixes were reviewed in the working tree before the final commit.

## Standards

The reviewer reported two documented-standard violations and one judgment-call smell with a concrete correctness consequence:

- **P1 — Explicit selected image not enforced.** Snapshot bytes were available through the worker, but a capture could submit a different selected URL or omit the original. This violated ADR-0042's exact selected-image and preservation rules.
- **P1 — Sanitization missed an excluded root.** Descendant-only HTML cleanup could export an editable root or content inside an excluded ancestor. The follow-up also identified a selection spanning from ordinary text into a form. This violated ADR-0045's form-content exclusion.
- **P2, possible Duplicated Code — Divergent Markdown escaping.** Record labels removed backslashes while index labels did not. A title ending in a backslash escaped the closing link delimiter and broke its index entry, contrary to ADR-0046's portable-link requirement.

All three are resolved. The controller requires the selected URL and exact original image bytes; the worker retains the original even when only an improvement is submitted. Snapshot extraction excludes roots, ancestors, and all excluded text within selected ranges. Records and index share one Markdown-label helper. Gateway, container, portable-vault, and real Chromium regression checks pass. The reviewer rechecked all three, including the previously failing spanning-selection fixture.

## Spec

The reviewer reported one finding:

- **P2 — A retried job displayed its prior failure.** The spec includes browser job retry. The receiver retained the failed result while changing the job to pending/running, and the UI preferred that old outcome. A retry therefore appeared failed while it was running.

Resolved by clearing the previous result when enqueuing retry and prioritizing active job states in the UI. The regression test pauses an active retry and checks its current state. The reviewer confirmed the fix and the selected-image enforcement. No additional missing requirements or unjustified scope were found. Documented manual stale-lock recovery after a hard crash remains a limitation rather than an unreported behavior.

Totals: Standards 3 findings, all resolved (highest original priority P1); Spec 1 finding, resolved (highest original priority P2). No remaining findings on either axis.
