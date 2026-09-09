# Artwork organization validation

## Scope

- Sticky Paintings inspector and stationary left navigation while gallery scrolls.
- Explicit Paintings image capture; text goes to Idea Sources only when selected there.
- General Settings entry above Open/Create Vault, available without a Vault.
- User-triggered researched artwork tagging through saved OpenAI provider configuration.
- Finite request/item/time limits, one active run, cancellation between requests, durable checkpoint/resume.
- Preserve original files and existing metadata; revision conflict protection and cited suggestions.

## Evidence so far

- Core capture suite: 10 tests passed; 454.5 MB peak, no swap.
- Focused native command routing regression: passed; 820.3 MB peak, no swap.
- Focused browser rerun: 11 tests passed; 698.7 MB peak, no swap.
- Actual 650px gallery scroll regression passes; screenshot `sidebar-scroll.png` is a mocked browser layout fixture, not a native image-performance measurement.
- Desktop and constrained screenshot baselines inspected; Settings text and actions remain readable.
- Full Rust suite: **172 passed**, zero failures; 2 GiB peak, no swap. See rust-test-log.md.
- Final full browser suite: **42 passed**, zero failures; 884.8 MB peak, no swap. See browser-tests-final.log.
- Startup-planning lock refinement: all-features check and focused enrichment regressions passed (7 tests). See rust-test-log.md.
- Production frontend build/typecheck passed; 314.2 MB peak, no swap. See frontend-build.log.

## Manual native check after build

1. Open Settings. Confirm the configured model is the intended API model ID. The saved key is never filled back into the UI.
2. Set a small first batch (one painting/request), then return to Paintings and click Refresh Item Records.
3. Confirm progress, new descriptive tags, and researched identity suggestions/provenance. Existing user metadata and preserved image bytes should remain intact.
4. If OpenAI rejects the model or key, the batch should pause promptly and show the failure; correct Settings before retrying. No model compatibility is claimed without a successful provider response.
5. Cancel a run. Resume should process pending paintings; after reopening the app, the saved run should remain resumable. An in-flight bounded request may finish before cancellation takes effect.
6. Edit a record externally during research: stale results should be rejected rather than overwrite that edit.

No paid requests or real credential-file inspection were performed during development. Monetary cost is unavailable; request/item/time controls are not dollar limits.
