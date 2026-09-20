# Discover nested destination folders

Status: complete

User request: immediately support existing nested folders as automatically discovered capture destinations. Investigate arbitrary manual rearrangement tracking only; do not implement it.

## Acceptance

- Discover existing ordinary directories recursively under the current `subvaults/` container, including folders created manually while GG is running. No registration, restart or migration to discover them.
- Expose relative paths such as `Photography/Historic` to the agent for automatic routing, including optional search/read of prior GG records. Uncertain captures retain Inbox behavior.
- Save/read/search/index nested records while preserving the existing `items/<id>` record layout and compatibility with existing vaults.
- Support explicit nested create/move/rename through management. Parent renames update descendant records. Undo preserves/restores empty directories and refuses to erase subsequent additions.
- Reject traversal and symlink destinations; ignore hidden folders and the reserved `items` record subtree. Bound path depth/length and document the limit.
- Existing installations, credentials and pairing remain valid. Rebuild worker and test; no live model calls or automatic service restart without assessing active state.
- Do not add automatic import of arbitrary loose files, polling, manual-move reconciliation or general sync.

## Validation

Focused filesystem tests, real Pi fixture routing to a manually created nested destination, full unit suite/typecheck, rootless worker build/container suite, independent Standards/Spec review.
