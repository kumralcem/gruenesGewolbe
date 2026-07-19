Status: ready-for-agent

# Permanently Delete a Trashed Item

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the deliberately destructive end of the Vault Trash lifecycle. Permanent deletion should be available only for an explicitly selected trashed item, show its Collection and Item Link impact, clean known canonical references, and avoid leaving a partially deleted state when cleanup cannot complete.

## Acceptance criteria

- [ ] Permanent deletion is available only from Vault Trash and requires explicit confirmation of the stable item ID.
- [ ] The confirmation view lists affected Collections and incoming Item Links before deletion.
- [ ] Confirmed deletion removes known Collection membership and incoming Item Link references before deleting the trashed Item Folder.
- [ ] A cleanup failure leaves the trashed Item Folder available and reports the operation as failed rather than silently leaving known dangling references.
- [ ] Successful permanent deletion updates derived state and appends an Activity Log event.
- [ ] The CLI requires explicit item identity and confirmation and reports reference impact and outcome in structured output.
- [ ] Tests cover cancellation, reference discovery, successful cleanup, cleanup failure, final deletion, derived-state rebuild, Activity Log behavior, and CLI safeguards.

## Blocked by

- .scratch/offline-archive-loop/issues/09-move-items-to-vault-trash-and-restore.md

