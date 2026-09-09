# Summary budget controls

Implemented the remaining Idea Source summary budget plumbing for issue 09.

- Added persisted `off`/`cheap`/`standard`/`deep` setting, defaulting to `standard`.
- Automatic capture and manual retry read the setting and pass it through the TypeScript adapter contract to the native `capture_idea_source` command.
- Native capture accepts an omitted mode as `standard` for compatibility and rejects invalid modes before extraction or saving.
- Summary mode validation now runs before provider status/configuration and source reads. `off` returns a skipped result without provider access or a request, while preserving the captured source.

Regression coverage was added to `apps/desktop/tests/browser/url-capture.spec.ts` for persistence, Off capture propagation/skipping, preserved source readability, and Deep retry propagation. Native unit coverage verifies Off and invalid modes return before invoking the provider and preserve source content. Validation: `pnpm --dir apps/desktop exec tsc --noEmit` and `cargo check -p gruenes-gewolbe-desktop --quiet` passed (existing dead-code warnings only). Heavy browser/native test suites were intentionally not run.
