Status: ready-for-human

# OpenAI Budgeted Metadata Suggestions

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first AI-assisted capture and enrichment workflow using OpenAI behind a provider boundary. Provider configuration should live in user-specific app state, not in the vault. AI budget modes should make cost and depth visible, and AI calls should send only the minimum files or cleaned text needed for the selected action.

AI output should produce summaries, tags, and metadata suggestions. High-confidence low-impact metadata may be accepted automatically; low-confidence, conflicting, or high-impact fields should become metadata suggestions that require review.

## Acceptance criteria

- [x] A user can configure OpenAI from the native app without writing API keys or provider settings into the Vault.
- [x] A user can choose an AI Budget Mode in the native app before AI-assisted capture or Enrichment.
- [x] Idea summarization sends cleaned text rather than raw HTML or unrelated vault records.
- [x] Image metadata suggestions send only the relevant image or preview needed for the selected action.
- [x] AI-generated summaries and tags are added when confidence and budget allow.
- [x] Low-confidence, conflicting, or high-impact metadata is staged as metadata suggestions with compact provenance.
- [x] Normal automated tests use fake providers and cover provider configuration, budget handling, minimal context boundaries, cost logging hooks, and suggestion acceptance rules.

## Blocked by

- .scratch/personal-archive-vault/issues/06-url-and-manual-fallback-capture.md
- .scratch/personal-archive-vault/issues/07-tags-collections-and-item-links.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/ai_enrichment.rs`, `apps/desktop/tests/ai_enrichment.rs`, and `apps/desktop/tests/ai_provider_config.rs`. AI enrichment accepts high-confidence suggestions only when they fill unknown placeholder metadata, records compact metadata provenance, stages conflicting suggestions into the review queue, and records additive better-file candidates with provenance without replacing the current primary file.

- 2026-08-20 dogfood correction: current evidence covers provider interfaces, user-state serialization, fake-provider behavior, and enrichment rules only. No live OpenAI provider, key-entry UI, Budget Mode control, or workbench Enrichment action is wired into the Tauri application. Those user-facing criteria remain agent-ready.

- 2026-09-08 implementation: native provider configuration/status and Responses summarization are wired through Idea Sources, with source-first persistence, retry, owner-only config permissions, and stale Vault/record/source guards. Capture uses the standard budget; a full native budget-mode control and exact live billing remain deferred. No paid request was made. See `.scratch/complete-idea-archive/VALIDATION.md` for automated evidence and native acceptance limits.

- Continuation from `4cc7703`: provider configuration is now in Settings, and artwork enrichment has bounded user-selected modes. Completing the remaining automatic Idea Source/retry budget control under `.scratch/spec-continuation/`; live configured-model success remains a manual acceptance gate, not a prerequisite for deterministic implementation tests.

- Current continuation: Settings now persist separate Idea Source Off/Cheap/Standard/Deep modes for capture and retry. Off preserves the source and skips before provider configuration access. Automated acceptance is recorded in `.scratch/spec-continuation/VALIDATION.md`; native GUI and the user-configured live model remain unverified.
