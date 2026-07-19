Status: ready-for-agent

# Edit Item Records Without Losing File Changes

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the structured Item Record editing path for common cleanup while keeping direct Markdown editing trustworthy. A save should be explicit, validated, atomic, preserve unknown content, detect external changes, and offer a separate confirmed readable-folder rename after metadata improves.

## Acceptance criteria

- [ ] Item Details provides a structured editor for title, creator, year, Saving Reason, summary, and Tags.
- [ ] Save validates the complete edit and performs one atomic canonical Item Record write.
- [ ] Saving known fields round-trips unknown frontmatter and user-authored Markdown sections unchanged.
- [ ] Tags continue to normalize through the vault-level Tag Registry while genuinely new Tags remain allowed.
- [ ] An external Item Record change made after the editor loads produces an Item Record Conflict instead of being overwritten silently.
- [ ] The conflict view supports reloading the external version and deliberate overwrite after the user reviews the conflict.
- [ ] Returning focus to the application refreshes externally changed records, and a manual refresh command is also available.
- [ ] Metadata changes may produce a readable Item Folder rename suggestion showing old and proposed paths without moving anything automatically.
- [ ] Confirmed rename preserves the stable item ID and uses a small unique suffix rather than overwriting an occupied path.
- [ ] Archive-core, Tauri-command, and browser-level tests cover validation, round-trip preservation, atomic saves, conflicts, refresh, rename confirmation, and collision handling.

## Blocked by

- .scratch/offline-archive-loop/issues/03-add-and-browse-first-painting.md

