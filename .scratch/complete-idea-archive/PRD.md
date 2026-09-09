Status: needs-info

# Complete the local idea archive

## Outcome

Save an image, tweet/post, or ordinary web page into a local Vault. For idea captures, preserve readable source text and a separate editable summary of the point being made, plus the Source Link and optional Saving Reason. Close the app, disconnect, reopen, read the preserved material, and find it through search. A URL alone does not meet this acceptance criterion.

This is a provisional investigation and delivery outline, not an approved implementation specification. The canonical planning artifact is now [Close browser tabs with confidence](map.md); its decision tickets take precedence over proposals below. This assessment follows the user's 2026-09-06 correction and ADRs 0019–0021. The narrower Offline Archive Loop PRD deferred URL extraction and live AI; those deferrals do not remove them from the overall product. Wayfinder was located at `/home/cem/Sync/Projects/DianeClaw/.agents/skills/wayfinder/SKILL.md` and used to chart that map.

User clarification: URL paste should let the user retire dozens of open browser tabs. Blog, tweet/post, and website content need an obvious destination in the left navigation. Pasted text is an acceptable extraction fallback, but the app is expected to summarize it. Manual summary entry alone is an intermediate capability, not product completion. The map includes both this full workflow and the image-responsiveness/test-freeze investigations.

## Evidence and limits

- README documents working image and offline Vault workflows. The user confirms image archiving works; this investigation did not re-run native acceptance.
- `apps/desktop/src/source_extractor.rs`: the native extractor supports Wikimedia and X image paths; generic hosts request fallback. It does not implement generic cleaned-text extraction.
- `apps/desktop/src/app.ts`: the normal `needs_manual_fallback` branch calls `captureManualFallback` with both `copiedText` and `copiedImage` null, then clears the capture form. That preserves provenance without preserving the source's content.
- `crates/archive-core/tests/capture.rs` demonstrates existing local text preservation at the core boundary. Reuse this capability rather than rebuilding the Vault.
- `.scratch/personal-archive-vault/issues/06-url-and-manual-fallback-capture.md` has checked acceptance boxes but later corrections explicitly distinguish fake/boundary evidence from actual native extraction. Treat those corrections as limitations, not completed product acceptance.
- Issue 09 in that same feature documents provider interfaces and fake-provider tests; live provider, configuration UI, budget controls, and native enrichment remain unwired.
- Playwright previously enabled fully parallel tests with no explicit worker count. The default is now one worker with fully parallel mode disabled. This bounds default test concurrency; it is not a demonstrated crash fix, a memory cap, or a prohibition on command-line overrides.
- The user clarified that testing freezes the machine rather than shutting it down, and the last occurrence was days ago. A previous-boot kernel journal query returned no entries; that does not meaningfully rule out any cause. RAM pressure is only a possibility. No browser was launched and the freeze was not reproduced.
- The user also reports noticeable delay between clicking an image and seeing it. Treat this as a distinct application responsiveness investigation, not as evidence for the test freeze's cause.
- Existing uncommitted capture changes were inspected as current behavior and left intact.

## Ordered delivery route

### 1. Establish verification that the computer can tolerate

Keep default Playwright execution at one worker. Do not run full browser suites, stress fixtures, native app builds, and Rust suites concurrently. First use type checking, browser test discovery without launch, and narrowly selected core tests with `CARGO_BUILD_JOBS=1` and `--test-threads=1`. These reduce concurrency; they do not impose a total memory limit. Before a browser run, inspect available memory and choose one workflow. Capture resource and kernel evidence if failure recurs; do not label an unobserved OOM as the cause. Native smoke remains a separate acceptance step.

Exit: safe defaults are checked; a small representative test can be run without machine failure, with actual evidence recorded. The configuration change alone does not close crash diagnosis.

### 2. Ship one complete manual idea capture

Let the user choose image capture versus idea/post/page capture; X URLs must not force an idea into Paintings merely because media exists. For an idea, accept URL, copied source text, optional Saving Reason, and an editable summary. Reuse the core text-preservation API and existing Item Records. Make missing text explicit and keep the draft available when automatic extraction fails. If link-only saving is offered, identify it as incomplete; do not report it as archived content. Support completing that same item later without manufacturing a duplicate.

Intermediate exit: paste a post and an article; inspect `source-copies/cleaned-text.md` and `record.md`; reopen offline and read the source and any manually entered summary; search finds the saved idea. No AI credentials are needed for this intermediate slice. Existing artwork capture still works. This is not the user's completed capture workflow until app-generated summaries are connected.

### 3. Add bounded extraction for ordinary pages

Implement a main-content extraction boundary and a bounded HTTP adapter for public pages. Preserve the full extracted main text, not just Open Graph descriptions or excerpts. Retain source attribution and distinguish complete content from truncated or unavailable content. Bound response bytes, elapsed time, redirects, and concurrent fetches; validate redirect destinations as well as initial URLs. Use saved HTML fixtures for deterministic tests. Dynamic, blocked, or incomplete pages retain the draft and lead to the working manual path from step 2.

Exit: representative article fixtures preserve paragraphs while excluding navigation; failures and oversized pages are explicit; one real public article passes native capture and offline reopen. No browser crawler is required.

### 4. Preserve posts as ideas

Add best-effort public post text extraction through the same boundary. Save post body, available author/date, and Source Link; preserve optional attached media alongside the idea when supported. Keep replies and surrounding discussion out by default, per the existing ADR. Do not substitute an image, unfurl title, login page, or excerpt for the full post. If public text cannot be obtained reliably, provide the pasted-text path without claiming automatic success.

Exit: text-only, text-with-image, and unavailable-post cases each produce an honest outcome; a manually preserved post remains readable offline. Live platform access is best-effort, not a guaranteed dependency.

### 5. Wire summarization into the real desktop workflow

Use the existing provider boundary, budget modes, provenance, and review rules. Existing ADRs select OpenAI first; changing to local model inference is a separate product decision, not required for local storage. Add user-specific provider configuration and an explicit summarize/enrich action. Preserve the source before calling a provider; failures must leave it readable and retryable. The summary should explain the source's central claim and useful supporting reasoning, distinguish the author's claim from established fact, and use the Saving Reason as user intent. Never invent a summary from a bare URL or replace source text with a summary. Preserve user edits under the existing conflict/review rules.

Exit: a real configured provider produces a persisted editable summary from the saved source; missing credentials, budget off, timeout, and provider failure preserve capture. Normal tests use fake providers. Remote summarization must be apparent to the user; local archive ownership remains independent of it.

### 6. Prove the product and repair completion claims

Run one acceptance path with an existing image, ordinary article, post, blocked source completed manually, and failed summarization retried later. Restart offline; inspect files, browse source content, edit a summary, rebuild derived state, and search. Record what was actually demonstrated separately from fixture, command, and browser-mock coverage. Update README and original capture/enrichment issue checkboxes against this evidence. Preserve unknown record content and existing archives; no unrelated rewrite or format migration is part of this route.

Exit: every example has the intended local content and usable summary, or is clearly incomplete with a recovery path. Passing mock tests alone is not completion.

## Next implementation scope

Proposed ordering remains subject to Wayfinder's destination interview. Before feature expansion, investigate the image-selection delay separately: measure click-to-feedback, command response, selected-image load/decode, and paint on representative existing images. Compare first selection with repeat selection; inspect whether full-resolution work blocks feedback and whether stale selection work is cancelled or ignored. Existing delayed-selection browser tests are controlled-response evidence, not proof of native performance. Agree a visible responsiveness criterion with the user and retain original files unchanged. Do not assume a cause or start a broad refactor from this report alone.

Then carry manual idea capture's UI, native command, core persistence, and offline reopening acceptance together. The persistence, text capture, metadata editing, and provider interfaces already supply much of the foundation.

## Checks during initial investigation

- `pnpm typecheck`: passed.
- `pnpm exec playwright test --list`: passed; discovered 31 tests across 12 files without starting a browser.
- Browser execution and native performance measurement: not performed.
