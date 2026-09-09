# Final UI scope

- Settings navigation sits above Open/Create Vault and works without an open Vault.
- Provider configuration moved into Settings; the saved API key is not returned to the UI.
- Settings persist budget mode, max paintings, max requests, and duration. Cost estimates are unavailable; no monetary cap is offered.
- Refresh Item Records starts artwork enrichment. Focus refresh remains a local snapshot reload.
- Progress includes cancellation, resume, counts, and failure details. Paused checkpoints are recovered on startup/Vault open.
- Progress callbacks merge current state. Completion/resume reload records; failed startup does not leave a forever-running indicator.
- Artwork details stay sticky with their own scrollbar; left navigation remains fixed while the gallery scrolls.
- Paintings manual fallback requires an image and calls captureArtworkFallback with copiedText null.

## Native contract

`startArtworkEnrichment(options, onProgress)` invokes `start_artwork_enrichment` with an `options` object. `resumeArtworkEnrichment(runId, onProgress)`, `cancelArtworkEnrichment(runId)`, and `artworkEnrichmentStatus()` use the corresponding snake-case Tauri commands. Progress arrives via `artwork-enrichment-progress`.

Options: budgetMode, maxItems, maxRequests, maxDurationSeconds, rerunCompleted. No maxCostCents field.

Regression coverage includes Settings without Vault, persisted controls, actual gallery scrolling, image fallback, start/cancel, recovery/resume, and provider rejection. See final browser log and VALIDATION.md for results.
