# UI workflow review against `fd5b602`

Reviewed 2026-09-07, read-only. No tests or builds were run.

## Findings

### High: Paintings link fallback still silently creates a link-only Idea Source

In `apps/desktop/src/app.ts:401-419`, the normal Paintings `data-source-link-capture` submit path receives `needs_manual_fallback`, then immediately calls `adapter.captureManualFallback` with `copiedText: null` and `copiedImage: null` whenever the native adapter exposes that method. The backend's manual fallback persists an Idea Source with no `source_copy` in this case. The UI then refreshes and closes the capture panel.

This contradicts the product acceptance rule that source preservation and summary generation are independent and that a link-only record must not be presented as preserved content. It also contradicts the progress note claiming the fallback draft is kept open. A blocked/unsupported link from Paintings should either remain open with the fallback form, or be explicitly offered as a link-only record with a clearly separate incomplete state and user action; the current automatic save loses the opportunity to paste text.

### Medium: switching Vaults retains an in-progress capture draft from the previous Vault

`activateOpenedVault` (`apps/desktop/src/app.ts:1040-1074`) spreads the old `state` into `nextState`, replaces the active vault/workbench, and refreshes provider status, but does not clear `capture_open`, `capture_fallback`, `capture_source_link`, `capture_copied_text`, or the capture title/reason. A user who starts a fallback in Vault A, opens Vault B, and submits the still-visible draft will save the source into Vault B. This is a cross-vault persistence hazard and should be resolved by clearing the draft on successful vault activation or explicitly re-associating it with the vault root and blocking submit after a switch.

## Reviewed behavior that looked aligned

- Idea-source capture keeps the draft open on `needs_manual_fallback` and preserves entered text in state.
- Idea-source list labels items with and without `source_copy` differently; the detail view does not call link-only content “preserved.”
- Summary notices distinguish generated, unavailable, skipped, and failed outcomes; provider status is redacted to configured/model state.
- Artwork selection uses `getItemDetails` when available and pauses new thumbnail preparation while a selection is pending.
- Native and mocked adapter contracts agree on the new reader/provider operations.
