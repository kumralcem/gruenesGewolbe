# SPEC review

Baseline: `4cc7703676ba4de0cf221e15366d7c3b484fb844`.

The final changes satisfy the requested behavior. Idea Source Settings exposes Off/Cheap/Standard/Deep, capture and retry propagate the persisted mode, Off returns before provider access, and capture preserves the source before attempting a summary. Issue metadata now records the implemented budget control; live native acceptance remains separate.

Thumbnail preparation now uses an isolated Vault for both gallery batches and per-item artwork enrichment. The shared gate keeps decode single-worker, and `record_thumbnail_failure_if_current` checks the Active Vault root and current Primary File before serialized canonical failure writes. The prior mutex blocker is resolved. Serial queueing through the gate is accepted for repeated native commands.

The frontend captures the preparation Vault root, stops applying resolved or rejected old-Vault results, and schedules preparation for the newly active Vault. Browser coverage exercises both old-Vault outcomes and confirms the stale error/status is not shown. The native damaged-image regression confirms stale failure handling and placeholder suppression.

The native-window click-feedback, WebView decode, and paint evidence remains open; the notes state that command measurements do not verify those boundaries.
