# Standards review

Baseline: `4cc7703676ba4de0cf221e15366d7c3b484fb844`
Scope: `apps/desktop/src/app.ts`, `contracts.ts`, `tauri-adapter.ts`, and the summary-budget portions of `main.rs`; thumbnail-worker changes/tests were excluded.

No documented standards violations found. The relevant ADRs are respected: budget mode is visible and persisted in app state (ADR0011/0027), capture remains usable without a provider (ADR0012), only cleaned source text is sent (ADR0029), and the source is captured before summary generation with optimistic revision checks retained (ADR0037/0039).

One judgement-call smell:

- **Repeated switches / duplicated validation** (`main.rs:110-124` and `main.rs:722-725`): the accepted budget literals are encoded independently in `summarize_live_with` and `capture_idea_source`. Adding or changing a mode requires synchronized edits, so the command can accept a value that the summarizer rejects (or vice versa). Centralize parsing in one helper (ideally reuse the core `parse_ai_budget_mode`) while preserving the early `Off` return.

The TypeScript handlers also repeat the same local-storage lookup and assertion at `app.ts:482` and `app.ts:783`; this is a minor **Duplicated Code** smell, not a correctness issue. The persisted key, default `standard`, adapter propagation, omitted native argument compatibility, early invalid-mode rejection, and provider-free `off` path all look correct from this diff.

## Parent integration review

Reviewed the final thumbnail core callback and native command changes: existing core entry points retain their default failure handling; both desktop decode paths share a separate gate; canonical Review Reason updates retain the command-state mutex and validate the current Vault and Primary File. Frontend response guards retain current selection state and discard old-Vault outcomes. No blocking integration finding remains. The minor budget-parser duplication is deferred.
