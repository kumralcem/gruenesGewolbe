# Final UI workflow review against `fd5b602`

Reviewed 2026-09-07. The review covered the current worktree diff from `fd5b602`, with emphasis on `apps/desktop/src/app.ts`, the shared contracts, the Tauri adapter, and existing Paintings interactions. No browser or build commands were run by this review.

## Findings

### High: asynchronous Idea Source operations can restore stale view state

Resolution: request epochs now invalidate stale reader, capture, and summary completions after navigation.

The Idea Source card, capture, and summary handlers all await adapter calls and then call `update({ ...state, ... })`, where `state` is the object captured when that render's event handlers were bound (`apps/desktop/src/app.ts:426-499`, `apps/desktop/src/app.ts:683-738`). The archive-view handler remains usable while these requests are pending. If a user starts reading or summarizing an Idea Source and switches to Paintings before the request resolves, the completion callback renders the old `active_view`, selected snapshot, and reader state again. The same race can overwrite a newer capture/view state after an in-flight capture resolves. These operations need a request/view generation check, or completion updates must merge against the current state and verify the active vault/item/view before applying.

### Medium: capture-time summary failures are reported as “not configured”

Resolution: capture now uses neutral copy: “Source saved; summary unavailable. Check settings or retry.”

`capture_idea_source` maps every non-generated, non-skipped `SummarizeIdeaSourceView` to the wire status `"unavailable"` (`apps/desktop/src/main.rs:362-371`). A configured provider that times out, rejects the request, or returns another error therefore reaches `captureSummaryNotice("unavailable")` and displays “Automatic summarization is not configured” (`apps/desktop/src/app.ts:1880-1884`). This is false and hides the retryable failure; the capture result needs a distinct `failed` status and the backend reason surfaced to the user.

### Medium: search results did not open the Idea Source reader

The shared search result handler previously called `refreshWorkbench` only. A result belonging to `Idea Sources` therefore stayed in the Paintings view and never called `readIdeaSource`, so searching for a saved source could not retrieve its preserved text. The handler now detects Idea Source IDs, switches to the Idea Sources area, loads the selected record, and reads its preserved content; a browser regression covers this path.

## Review coverage that looked aligned

- Empty Idea Source states and incomplete link-only records are visibly distinguished.
- Manual fallback validation prevents saving an empty source copy.
- Vault activation now clears capture and reader state.
- Summary retry after an explicit `summarizeIdeaSource` failure preserves the source and reports the backend reason.
- The lightweight `getItemDetails` selection path is guarded against stale artwork responses; a focused browser case was added in `apps/desktop/tests/browser/workbench-interactions.spec.ts`.

## Narrow UI/test changes made during this review

- `apps/desktop/src/styles.css`: made capture labels and controls full-width stacked fields, gave the source textarea a usable minimum height, and anchored the capture close button to its panel.
- `apps/desktop/tests/browser/workbench-interactions.spec.ts`: added coverage that selection uses `getItemDetails`, avoids rebuilding the workbench snapshot, and ignores a slower stale selection response.
- `apps/desktop/src/app.ts`: search results for Idea Sources now open the reader and load preserved text.
- `apps/desktop/tests/browser/workbench-interactions.spec.ts`: added search-to-reader coverage.
- `apps/desktop/src/app.ts`: review-queue Idea Sources now use the reader path as well.
- Regression added: `ignores a delayed Idea Source reader after switching to Paintings` in `apps/desktop/tests/browser/workbench-interactions.spec.ts`.
