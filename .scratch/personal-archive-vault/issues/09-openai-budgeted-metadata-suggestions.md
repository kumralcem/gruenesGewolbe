Status: ready-for-agent

# OpenAI Budgeted Metadata Suggestions

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the first AI-assisted capture and enrichment workflow using OpenAI behind a provider boundary. Provider configuration should live in user-specific app state, not in the vault. AI budget modes should make cost and depth visible, and AI calls should send only the minimum files or cleaned text needed for the selected action.

AI output should produce summaries, tags, and metadata suggestions. High-confidence low-impact metadata may be accepted automatically; low-confidence, conflicting, or high-impact fields should become metadata suggestions that require review.

## Acceptance criteria

- [x] A user can configure OpenAI for the app without writing API keys or provider settings into the vault.
- [x] A user can choose an AI budget mode before AI-assisted capture or enrichment.
- [x] Idea summarization sends cleaned text rather than raw HTML or unrelated vault records.
- [x] Image metadata suggestions send only the relevant image or preview needed for the selected action.
- [x] AI-generated summaries and tags are added when confidence and budget allow.
- [x] Low-confidence, conflicting, or high-impact metadata is staged as metadata suggestions with compact provenance.
- [x] Normal automated tests use fake providers and cover provider configuration, budget handling, minimal context boundaries, cost logging hooks, and suggestion acceptance rules.

## Blocked by

- .scratch/personal-archive-vault/issues/06-url-and-manual-fallback-capture.md
- .scratch/personal-archive-vault/issues/07-tags-collections-and-item-links.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/ai_enrichment.rs`, `apps/desktop/tests/ai_enrichment.rs`, and `apps/desktop/tests/ai_provider_config.rs`. AI enrichment also records additive better-file candidates with provenance without replacing the current primary file.
