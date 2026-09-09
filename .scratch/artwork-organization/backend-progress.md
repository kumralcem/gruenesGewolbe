# Backend progress

## 2026-09-09

- Read `CONTEXT.md` and ADRs 0007, 0011, 0012, 0014, 0015, 0025, 0026, 0027, 0028, and 0029.
- Verified the current OpenAI Responses API supports image inputs, built-in `web_search`, `max_tool_calls`, bounded output tokens, `store: false`, and including web-search sources.
- Coordinated with root to keep Paintings capture routing and `archive-core/src/lib.rs` changes in root's ownership.
- Requested a narrow core apply-response seam so the batch can send only a derived Thumbnail Preview and avoid reading/uploading the full preserved file.
- Backend design: durable hidden-Vault checkpoint; one artwork per Responses request; item/request/duration limits; cancellation checked between requests; Active Vault + Item Record revision + Thumbnail Preview fingerprint rechecked before applying; successful unchanged records skipped on resume; explicit rerun supported; per-item failures retained.

## Current work

- Implement `apps/desktop/src/artwork_enrichment.rs` with provider request/response parsing, source URL validation, checkpointing, planning, and tests.
- Add native commands and TypeScript adapter wiring after agreeing the exact UI contract.

## Implemented

- Added a real image-aware OpenAI Responses provider using the runtime-configured app model, a derived PNG Thumbnail Preview (`detail: low`), structured output, and the hosted `web_search` tool.
- Factual `title`, `creator`, and `year` suggestions are discarded unless their URLs occur in the response's actual web-search source list. Descriptive visual tags can come from the bounded image alone. Provider output never supplies a Summary or file move/replacement.
- Added an atomic hidden-Vault checkpoint containing every painting. Each invocation enforces item, request, total-duration, and 90-second per-request limits. Requests are counted durably before being sent. Successful unchanged items are fingerprint-verified and skipped without consuming the next invocation's item/request allowance.
- Added cancellation between requests, a single-flight guard, restart status that converts orphaned `running` state to `paused`, explicit resume/rerun, early pause on provider rejection, and visible per-item failures.
- Planning snapshots the Active Vault and Item Record revision without reading all images. A real Thumbnail Preview is prepared for one item at a time. The current thumbnail path is re-derived by item ID; persisted checkpoint paths are never trusted for uploads. Record revision and thumbnail fingerprint are checked again before applying.
- Large-Vault planning holds the global desktop state lock only long enough to copy the Active Vault root and provider configuration. Metadata enumeration and Item Record scans use an isolated read handle, so the UI remains responsive; planning also observes cancellation between items.
- Added a revision-checked core apply seam. It preserves filled/user fields through the existing confidence-and-review rules, merges tags, keeps source and primary files unchanged, and never moves item folders.
- Added the Paintings manual fallback TypeScript adapter alongside the batch start/cancel/resume/status API.

## Rust verification log

Command: `scripts/check-bounded.sh cargo test --workspace --all-features`

- Result: success; exit status 0.
- 172 tests passed, 0 failed (12 CLI, 93 core, 67 desktop/runtime, including unit, integration, and doc-test targets).
- New coverage passed for checkpoint resume, all-paintings planning beyond a one-item invocation limit, unchanged-success fingerprint gating, bounded options, structured-response source allowlisting, stale Item Record rejection, and preserving user title/notes/original bytes.
- Service runtime: 2m 32.810s.
- CPU time: 2m 25.153s.
- Memory peak: 2 GiB; swap: 0 B.
- No live OpenAI request was made.
- Post-review verification after the unlocked large-Vault planning change: all-features desktop compile passed; focused bounded enrichment tests passed (7 selected tests, 0 failures; 1m 15.504s; 2 GiB peak; no swap).
