# Artwork organization follow-up

User authorized implementation, lower-cost agents, review, commit and push.

- Root: make selected capture area authoritative; regression tests and final integration review.
- Luna UI agent: sticky details, general Settings tab, bounded enrichment controls.
- Sol backend agent: image-aware researched enrichment using configured app provider, durable checkpoints, cancellation, revision guards.
- User clarified Luna is an app model/provider slot, not merely a coding model preference. Compatibility must be checked against actual saved configuration; no paid calls during development.
- Preserve legacy captures; do not infer migrations or delete them.
- All heavy verification uses scripts/check-bounded.sh, one run at a time.

Status: implementation in progress. See backend-progress.md and ui-progress.md.

## Capture verification

- Added explicit Paintings-only URL capture and image fallback commands. Text extraction returns a fallback prompt rather than creating an Idea Source. Generic historical core capture API remains compatible.
- Targeted desktop regression passed under bounded runner: 1 test, 820.3 MB peak, zero swap. Verifies text URL fallback, rejection of text-only artwork, and preserved pasted-image bytes in Paintings.
- User confirmed the model/API key were entered through this app’s existing Settings; no custom endpoint requested. Agents have not inspected the saved API key. Compatibility will be checked by the provider at user-triggered runtime.

## Integration review

- Independent Luna capture review found no blockers; report capture-review.md.
- Core capture suite: 10 passed, 454.5 MB peak, zero swap.
- Fixed UI review findings: saved limits previously ignored on Paintings page; sticky CSS overridden by later rule; redundant Settings state; hidden/non-enforceable estimated cost cap removed.
- Settings now explains configured-provider use and unavailable monetary cost, with finite request/item/time limits.
- Backend checkpoint retains all paintings; batch limits apply per invocation. Completed unchanged entries remain skippable across starts.
- Full browser regression suite currently running; backend native loop under implementation.

## UI validation

- Full first pass: 36/40 browser tests passed; two old navigation test edits targeted the wrong occurrence and two layout baselines needed the intentional Settings rail update. Corrected the selectors, reviewed mobile layout and fixed action alignment.
- Focused rerun: 11/11 passed, 698.7 MB peak, no swap. Includes actual 650px gallery scroll with stationary left navigation and visible sticky inspector. Updated screenshot baselines reviewed visually. Mocked screenshot sidebar-scroll.png proves layout only, not native image loading.
- Added saved checkpoint status on startup/Vault open and visible failure details; callbacks merge current UI state. Final verification pending native integration.

## Final verification checkpoint

- Full Rust suite passed: 172 tests, 0 failures, bounded 2 GiB peak/no swap.
- Full browser suite passed: 42 tests, 0 failures, 884.8 MB peak/no swap.
- Final native refinement moves metadata planning outside the global app-state lock, with cancellation between records. Focused compile/test of that refinement running; frontend production build next.
- Independent backend/capture reviews have no remaining blockers. No live paid/provider requests or real credential-file reads.

## Completed

Implementation and review complete. Final native all-features check plus seven focused enrichment tests passed after the planning-lock refinement. Production frontend build/typecheck passed under the bounded runner (314.2 MB peak, no swap). Full suites passed: 172 Rust and 42 browser tests. Commit/push performed by root after this checkpoint; consult git log for the commit ID.

Live Luna/API compatibility remains untested because no paid requests were made. Start with a small user-triggered batch to verify the saved model slug. Durable files remain in this repository.
