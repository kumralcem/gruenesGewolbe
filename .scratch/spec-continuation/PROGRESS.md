# Spec continuation

Baseline: `4cc7703676ba4de0cf221e15366d7c3b484fb844`.

User authorized continuing ready spec issues, reviewing, committing, and pushing. Existing preference: use lower-cost agents and preserve durable work in the repo.

## Selected work

1. Finish the explicit AI Budget Mode requirement in personal-archive-vault issue 09 for Idea Sources: configurable Off/Cheap/Standard/Deep for automatic summary and retry, source-first persistence regardless of budget. Luna implements this slice.
2. Investigate complete-idea-archive issue 04 using bounded native-command measurements. Sol measures the state-lock/thumbnail boundary and applies a narrow responsiveness fix when supported. Native-window latency remains a separate acceptance gate.

No new product decision is required for these slices: provider direction, local preservation, visible Settings, and budget modes are already specified.

## Remaining decisions / acceptance

- Preserved-artwork viewer issue 11 still asks for presentation and zoom/pan choices. Do not silently choose a viewer or decode huge originals into the detail pane.
- Whole-product acceptance still needs representative real sources, a native-window smoke run, and a live configured-model response. Mock/browser tests and command measurements do not substitute for those.
- Legacy link-only captures should stay intact until a completion/migration workflow is explicitly specified. Do not infer their destination.
- Historical machine-freeze cause remains unknown. All heavy verification uses the existing 2 GiB, no-swap, one-worker/sequential runner.

## Validation plan

Targeted budget and native concurrency regressions, independent code review against the baseline, then the full Rust/browser suites and frontend build sequentially. No saved credential-file inspection or paid provider calls.

## Integration findings

- Summary budget browser regressions passed (6 tests); native Off/invalid tests now remove their generated fixture provider configuration to catch configuration-first regressions.
- Baseline with 64 real PNG records reproduced selection blocked for the duration of thumbnail preparation (~25 seconds); first/repeat metadata requests alone were ~100 ms. See selection-measurement.md for exact scope.
- Worker change keeps image decode outside the UI-state lock, canonical failure updates guarded, and decoding serialized independently. Per-item artwork enrichment now uses the same boundary and worker gate.
- Root added browser Vault-scope guards so old preview successes/errors cannot leak into a newly-opened Vault; preparation resumes for the new Vault. Two controlled browser regressions cover both outcomes.
- Standards review found no documented violations; spec review identified the second enrichment decode path, resolved and confirmed by final spec review.

## Completed

Implementation and reviews complete. All 176 Rust tests, 45 browser tests, and the TypeScript/production build passed under the bounded runner. See VALIDATION.md. Issue 09 is ready for human native/live-model acceptance; measurement issue 04 remains open for native GUI boundaries. No further product decision was needed for this slice.
