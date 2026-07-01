Status: ready-for-agent

# Activity Log and Agent CLI

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the audit and agent-access path around the archive core. The activity log should append capture, import, enrichment, error, index rebuild, and AI cost events as secondary debugging material. The minimal CLI should give agents and scripts enough access to validate vaults, import Paintings, rebuild indexes, search metadata, and inspect item records without driving the GUI.

The activity log must not become the source of truth. A vault should remain reconstructable from item records, preserved files, collection files, tag registry, and other visible canonical files.

## Acceptance criteria

- [x] Capture, import, enrichment, error, index rebuild, and AI cost events append to an activity log.
- [x] Deleting the activity log does not prevent the vault from opening, rebuilding indexes, or searching canonical records.
- [x] The CLI can validate/open a vault, import Paintings, rebuild indexes, search metadata, and inspect item records.
- [x] CLI output is structured enough for agents and scripts to consume without parsing UI text.
- [x] The app exposes enough error detail for a user to connect UI failures to activity log entries.
- [x] Tests verify activity log append behavior, noncanonical status, CLI command behavior, and reconstructability from canonical vault files.

## Blocked by

- .scratch/personal-archive-vault/issues/03-import-paintings-folder.md
- .scratch/personal-archive-vault/issues/04-rebuildable-metadata-search.md
- .scratch/personal-archive-vault/issues/06-url-and-manual-fallback-capture.md
- .scratch/personal-archive-vault/issues/09-openai-budgeted-metadata-suggestions.md

## Comments

Implemented with TDD. Evidence: `crates/archive-core/tests/activity_log.rs`, `crates/archive-core/tests/ai_enrichment.rs`, `crates/archive-cli/tests/inspect_item.rs`, `crates/archive-cli/tests/validate.rs`, and `apps/desktop/tests/error_activity.rs`.
