# Validation record

Final automated validation: 2026-09-08.

## Passed

- `cargo test --workspace --all-features` through the bounded runner: **163 tests passed**, zero failed/ignored across 47 reported test suites (including empty/doc-test suites). Native-feature compilation and concurrency regressions included. Service runtime 2m 13.860s; peak 2 GiB; no swap. Full output: [rust-tests.log](rust-tests.log).
- `pnpm test` through the bounded runner: **36 passed**, zero failed, 40.6s. Peak approximately 1 GiB; no swap. Includes capture, fallback, provider setup, search-to-reader, stale navigation, lightweight artwork selection, and existing archive regressions. Full output: [browser-tests.log](browser-tests.log).
- `pnpm build` through the bounded runner: TypeScript check and Vite production build passed; peak 220.1 MiB, no swap. Output: [frontend-build.log](frontend-build.log).
- `git diff --check`: passed.
- Visually inspected desktop/constrained navigation baselines, [capture form](evidence/idea-source-capture.png), and [source reader](evidence/idea-source-reader.png). Corrected cramped form fields and close-button positioning before the final run.

## Review results

Root plus lower-cost Sol/Luna agents reviewed the implementation. The identified correctness findings were fixed: empty fallback saves, stale UI completions, search results not loading preserved text, capture drafts crossing Vault activation, widened image redirects, unsafe/unbounded source-copy reads, summary writes after source/record/Vault changes, misleading summary failure copy, and unknown billing logged as free. Native tests inject responses to verify failed summaries preserve source content and stale results are rejected. See backend-progress.md and ui-final-review.md for detail.

## Acceptance limits and next work

- Browser workflow tests use controlled Tauri adapter responses; they do not prove native-window interaction or live website access. Native-window smoke and representative real public article capture remain unperformed; follow `docs/idea-archive-smoke.md`.
- No paid OpenAI request was made. The app has a live provider implementation and setup UI; deterministic tests verify parsing/limits and persistence/failure behavior. Exact live billing is unknown and omitted from numeric cost logs.
- Image selection avoids a full workbench rebuild and pauses newly scheduled preview work while selecting. Native latency remains unmeasured. An already-running thumbnail decode still holds shared state, so the reported lag and historical freeze are not declared fixed.
- URL extraction is best-effort; blocked, dynamic, ambiguous, or oversized pages need pasted text. Public-host checks reject literal private/loopback addresses, including IPv6, but do not pin DNS resolution against private targets.
- Resource-limited tests passed without exhausting the desktop's swap. This establishes a bounded test procedure, not the historical freeze's root cause.

## Subsequent command responsiveness fix

The former thumbnail command-state lock limitation is superseded by `.scratch/spec-continuation/selection-measurement.md`: both gallery and enrichment decode outside that lock through a sequential gate. Native GUI timing and the historical freeze cause remain unverified.
