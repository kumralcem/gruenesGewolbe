Status: ready-for-human
State: open
Assignee: unassigned
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Parent: ../map.md
Blocked by: 03-make-idea-sources-visible.md, 04-measure-image-selection.md, 05-choose-summary-workflow.md

# Define proof that the whole archive works

## Question

Which representative sources, responsiveness criteria, and end-to-end demonstrations are sufficient to hand off an implementation route without repeating the earlier partial-completion problem?

## Exchange to conduct

Choose representative blog, post, and website examples with the user, including one requiring text fallback. Carry an existing image through the same native acceptance session. Require local content inspection, generated summary, app restart offline, search and retrieval, and honest incomplete/retry cases. Adopt measured image-selection criteria and bounded test procedures from their tickets. Distinguish a source that cannot be fetched automatically from a failed archive product when the agreed manual route works.

## Resolution evidence

A whole-product acceptance checklist and sequence of implementation slices, each spanning its necessary UI, native adapter, persistence, and verification. Record native evidence separately from core tests and mocked browser coverage. Reconcile original capture/enrichment completion claims only against demonstrated behavior.
