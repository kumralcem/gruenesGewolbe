Status: ready-for-agent

# Edit Item Records Without Losing File Changes

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the structured Item Record editing path for common cleanup while keeping direct Markdown editing trustworthy. A save should be explicit, validated, atomic, preserve unknown content, detect external changes, and offer a separate confirmed readable-folder rename after metadata improves.

## Acceptance criteria

- [x] Item Details provides a structured editor for title, creator, year, Saving Reason, summary, and Tags.
- [x] Save validates the complete edit and performs one atomic canonical Item Record write.
- [x] Saving known fields round-trips unknown frontmatter and user-authored Markdown sections unchanged.
- [x] Tags continue to normalize through the vault-level Tag Registry while genuinely new Tags remain allowed.
- [x] An external Item Record change made after the editor loads produces an Item Record Conflict instead of being overwritten silently.
- [x] The conflict view supports reloading the external version and deliberate overwrite after the user reviews the conflict.
- [x] Returning focus to the application refreshes externally changed records, and a manual refresh command is also available.
- [x] Metadata changes may produce a readable Item Folder rename suggestion showing old and proposed paths without moving anything automatically.
- [x] Confirmed rename preserves the stable item ID and uses a small unique suffix rather than overwriting an occupied path.
- [x] Archive-core, Tauri-command, and browser-level tests cover validation, round-trip preservation, atomic saves, conflicts, refresh, rename confirmation, and collision handling.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-artwork-saved-item.md

## Comments

Implemented a complete structured Item Record edit through archive-core, typed Tauri commands, and the browser workbench. Saves validate all owned fields before atomically replacing `record.md`, preserve unknown YAML and user-authored Markdown, normalize aliases through the Tag Registry, register genuinely new Tags, rebuild derived search, and carry an optimistic record revision. Stale saves return an Item Record Conflict with the external version; the editor preserves its draft for reviewed overwrite or can reload canonical disk state. Manual and focus refresh are available, with dirty drafts retained across focus refresh. Readable Item Folder suggestions show current and proposed paths, never move on metadata save, and require a separate confirmation that preserves the stable item ID and chooses a small collision suffix. Evidence: `crates/archive-core/tests/item_record_editing.rs`, `apps/desktop/tests/item_record_editing.rs`, and `apps/desktop/tests/browser/item-record-editing.spec.ts`; the complete Rust workspace and 14 browser tests pass.
