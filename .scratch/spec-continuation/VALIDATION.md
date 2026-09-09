# Spec continuation validation

Baseline: `4cc7703676ba4de0cf221e15366d7c3b484fb844`.

## Summary budgets

Focused browser source-capture suite: 6 passed, zero failures. Includes selecting Off in Settings, persistence across reload, captured source remaining readable without summary, and Deep being passed on retry. Runner peak 1009.6 MB, zero swap; `budget-browser.log`.

Native Off/invalid regression tests remove their generated test provider configuration before calling the native summary seam. No real credential files or provider requests are used.

## Image selection

See `selection-measurement.md` for fixture definition, native command measurements, and the reproduced failing baseline. First details request is not a cold filesystem measurement. The fixture measures command/mutex latency, not native-window click-to-paint latency.

## Final gates

- Focused native regression: matched 64-image concurrency and stale-Vault failure guards passed; see `selection-measurement.md`.
- Full Rust suite (`cargo test --workspace --all-features`): 176 passed, zero failures; 2m 5s service runtime, peak 2 GiB, zero swap; `rust-tests.log`.
- Full browser suite: 45 passed, zero failures; 50s service runtime, peak 1 GiB, zero swap; `browser-tests.log`.
- TypeScript check and production build: passed; peak 291.2 MB, zero swap; `frontend-build.log`.
- Standards/spec and parent integration review completed; the enrichment lock finding was fixed and re-reviewed. No blocking findings remain. See `standards-review.md` and `spec-review.md`.

No native GUI smoke, live website capture, or paid model request is claimed. Historical whole-machine freeze cause remains unknown.
