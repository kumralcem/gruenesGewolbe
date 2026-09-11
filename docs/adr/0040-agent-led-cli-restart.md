---
status: accepted
---

# Restart Around an Agent-Led CLI

GG uses Pi's SDK for a small agent-led CLI, with no Pi fork required. The old Rust core and Tauri shell are removed on the restart branch; their code remains in Git history. Existing live vaults are not migrated or modified by this restart.

Capture intent is explicit: `art`/`painting` preserves one selected image and identification metadata, while `idea` preserves source text and useful ideas/instructions. Only the user defines destinations. The archive keeps a shallow item layout, original files, and an Obsidian-compatible Markdown record without a database. See [visual capture](0041-agent-classified-visual-capture.md), [selected images](0042-one-selected-image-per-visual-capture.md), and [records](0046-obsidian-compatible-records.md).

The CLI accepts multiple URLs or newline-separated stdin. Each capture has an independent outcome; failures are reported and processing continues. Inaccessible sources are skipped without an incomplete item or a decision-queue entry. Ideas use existing transcripts only; if none is available, skip and notify. Do not download audio or transcribe it.

Summaries are written for retrieval by remembered problems and concepts. Instructional material retains actionable steps and conditions, removing surrounding narrative. Optional focus narrows the summary while the source text remains preserved. Known publication and capture dates are visible; there is no active rechecking or expiry system.

`search` scans local files without a model. `ask` uses bounded search/read tools to return relevant existing files, without editing records or browsing the web. Provider selection supports OpenAI and OpenRouter with arbitrary compatible model slugs and private configuration. A convenience settings wizard remains planned.

The agent can use scripts and ordinary CLI tools inside its sandbox. The controller owns keys and archive writes; [ADR-0043](0043-isolated-agent-with-trusted-controller.md) defines the boundary. The future hosted interface reuses these operations as described in [ADR-0044](0044-private-hosted-interface-over-file-vault.md).

The initial public-only source-access decision was superseded by the user's explicit browser-extension request on 2026-09-11. Direct URL capture stays public; [ADR-0045](0045-browser-session-capture.md) permits collecting content in the user's authenticated browser without exporting its credentials.
