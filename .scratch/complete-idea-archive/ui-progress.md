# Idea Sources UI progress

## Current implementation

- Added first-class `Paintings` and `Idea Sources` destinations to the left navigation.
- Added a dedicated Idea Sources workspace with URL, optional title, Saving Reason, and optional pasted source text capture.
- Extraction fallback keeps the entire capture draft open and requires pasted source text; it no longer silently saves an empty link-only fallback.
- Added a source list with explicit `Readable source preserved` and `Source text still needed` states.
- Added an Idea Source reader with saved summary, Saving Reason, Source Link, and locally preserved source text.
- Added honest summary states and retry controls. Generated summaries are only claimed from backend status; missing provider configuration is shown as unavailable.
- Added local OpenAI provider setup UI with password input and optional model.
- Artwork selection feature-detects the lightweight `getItemDetails` command and falls back to the old snapshot call for test adapters. Thumbnail preparation pauses while an artwork selection request is pending.
- Added focused browser scenarios for manual text fallback, configured automatic summarization, provider setup, protection against empty legacy fallback, and preserved artwork URL capture.
- Independent review found and fixed the remaining empty-fallback path in the Paintings capture. Extraction failure now always retains the draft, and manual fallback refuses to save until text or an image is supplied.
- Opening another Vault clears capture and reader state so a draft cannot carry across Vault roots.

## Validation

- `apps/desktop: pnpm typecheck` passed after final shared-contract alignment and the review fixes.
- `git diff --check` passed for the owned UI files.
- The first bounded browser invocation accidentally selected the full suite because of an extra argument delimiter. It remained capped at roughly 1 GiB peak memory with zero swap; 27 tests passed and four failed. Two failures were expected layout baselines after the new navigation. The two focused locator failures were corrected before the targeted rerun.
- The corrected bounded run passed all 5 focused browser cases in 7.5 seconds at 671 MiB peak memory with no swap. It produced screenshot attachments for the manual fallback and completed Idea Source reader for visual review.

## Next steps

1. Update the two visually approved Paintings layout baselines for the new left navigation while holding the bounded-test lock.
2. Complete native smoke evidence for a real public article, manual fallback, provider summary, and offline reopen.

## Final integration validation, 2026-09-08

All final automated checks passed; see [VALIDATION.md](VALIDATION.md). Both navigation baselines and capture/reader screenshots were inspected. Earlier pending automated checks in this historical log are superseded by that result. Native smoke and measured native responsiveness remain outstanding.
