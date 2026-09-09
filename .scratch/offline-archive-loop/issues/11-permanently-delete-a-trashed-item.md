Status: ready-for-human

# Permanently Delete a Trashed Item

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the deliberately destructive end of the Vault Trash lifecycle. Permanent deletion should be available only for an explicitly selected trashed item, show its Collection and Item Link impact, clean known canonical references, and avoid leaving a partially deleted state when cleanup cannot complete.

## Acceptance criteria

- [x] Permanent deletion is available only from Vault Trash and requires explicit confirmation of the stable item ID.
- [x] The confirmation view lists affected Collections and incoming Item Links before deletion.
- [x] Confirmed deletion removes known Collection membership and incoming Item Link references before deleting the trashed Item Folder.
- [x] A cleanup failure leaves the trashed Item Folder available and reports the operation as failed rather than silently leaving known dangling references.
- [x] Successful permanent deletion updates derived state and appends an Activity Log event.
- [x] The CLI requires explicit item identity and confirmation and reports reference impact and outcome in structured output.
- [x] Tests cover cancellation, reference discovery, successful cleanup, cleanup failure, final deletion, derived-state rebuild, Activity Log behavior, and CLI safeguards.

## Blocked by

- .scratch/offline-archive-loop/issues/09-move-items-to-vault-trash-and-restore.md

## Comments

- 2026-08-20 implementation: existing Vault Trash snapshots already exposed affected Collections and incoming Item Links. Added exact stable-ID confirmation and permanent deletion through archive-core, structured CLI output, Tauri command state/runtime registration, typed frontend adapter, and the browser workbench.
- ADR-0036 verification: a malformed possible-referrer Item Record blocks reference cleanup, remains a localized Vault Problem, and leaves the trashed Item Folder untouched.
- Verification: `cargo test --workspace` passed; `pnpm build` passed (TypeScript typecheck plus Vite production build); `pnpm exec playwright test` passed 21/21 browser tests. Focused archive-core, CLI, desktop command-state, and Vault Trash browser tests cover refusal/cancellation, impact discovery, cleanup, malformed-record cleanup failure, derived-index rebuild, Activity Log append, final removal, and CLI safeguards.
