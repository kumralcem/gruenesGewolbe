Status: ready-for-agent

# Move Items to Vault Trash and Restore Them

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build recoverable removal through visible canonical Vault Trash. A user or agent should be able to move an Item Folder out of active browsing, inspect its trashed state through retained references, and restore it to its Home Subvault without overwriting an occupied path.

## Acceptance criteria

- [ ] Moving a Saved Item to Vault Trash relocates its complete Item Folder under `trash/<home-subvault>/` without deleting canonical content.
- [ ] Trashed items are excluded from normal gallery browsing, search results, collection counts, and the Review Queue.
- [ ] Collection membership and incoming Item Links remain canonical and visibly identify that their target is in Vault Trash.
- [ ] The workbench provides a Vault Trash view with item details and a restore action.
- [ ] Restore returns the item to its Home Subvault and preserves its stable item ID and retained references.
- [ ] Restore never overwrites an occupied Item Folder and uses a small unique suffix when necessary.
- [ ] Vault Trash has no automatic purge behavior.
- [ ] Structured CLI commands can move and restore an explicitly identified item.
- [ ] Tests cover move, active-view exclusion, retained references, restore, path collision, restart, derived-index rebuild, and CLI behavior.

## Blocked by

- .scratch/offline-archive-loop/issues/08-keep-vault-usable-around-malformed-records.md

