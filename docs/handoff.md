# Next-session handoff

Updated 2026-09-20. Current behavior and setup: [README](../README.md), [architecture](adr/0051-current-architecture.md), [validation](validation.md). Keep this file in the project; historical interviews/reviews remain in Git.

## Authorization and decisions

The user authorized fixing the import and implementing inherited folder instructions and upload/download interfaces. They explicitly rejected model-controlled browser clicks/scrolling for the extension. The larger browser capture redesign below is still design work, not permission for browser interaction.

Implemented in this iteration:

- CONNECT sockets get error handling before validation/DNS; rejected broken pipes no longer terminate the controller. Regression reproduces the original EPIPE and checks the next request. This does not prove every possible transport failure is fixed.
- Repeated identical image previews are removed from model context; public link/image candidates are bounded; local image research gets at most four browse/fetch reads and switches to save-only tools at 70% of the conservative input bound. Evidence and conversation history are retained. Completed captures no longer need another paid completion to announce success. Full originals remain intact. These bounds are not general long-document compaction.
- Root → parent → child `CAPTURE.md` inheritance, settings editor/effective view, bounded files, revision checks/history and saved policy chains for retries. Nonempty folder instructions must be read before saving. Source pages cannot edit policy or expand capabilities.
- Settings-page image picker/drop import through the existing durable queue. Archive search and portable `.tar.gz` downloads, also `gg download ID --to FILE.tar.gz [--originals-only]`. Management pairing required for archive access; capture-only tokens remain sufficient for uploads. Bundles exclude history/internal state; current bundle limit 128 MB. Browser buffers one bundle, CLI/server stream it. No new daemon or dependency.
- Uncertain creator/year values cannot be copied verbatim into confident artist/date tags (normalized punctuation/case). This is a narrow consistency check, not factual authentication.

## Import reliability follow-up

Regression tests cover proxy EPIPE handling and closing research before the model-input bound. Live local-image imports still exceed the 64,000-token input bound in some cases despite the early research cutoff. Larger runs need checking for per-image failures; a running batch is not proof that every earlier image saved. Use `gg usage` and `gg usage history`, inspect the run's own output, and compare input content hashes against preserved records. Respect existing grant expiry and provider pauses. Re-running an import skips saved source hashes and retries missing records.

Remaining product issue: local-image interpretation still has hardcoded artwork research and routing preferences, even for non-art images. Move domain-specific preferences into destination policies while retaining general evidence/uncertainty checks. See [supported formats and specialization](files.md). Non-image file import is a separate feature, not a prompt change.

Keep deployment-specific paths, service names, grant IDs, collection inventories and review operation IDs out of public project docs.

## Passive browser capture design

**No model clicks, scrolling, navigation or browser-control loop.** The extension reads a snapshot after an explicit click/shortcut. No required region/type/image selection. Optional instructions stay in the popup.

Recommended next step: a generic structured observation of already-loaded page content:

1. Use viewport intersection, semantic containers (article/main/figure), headings and link targets as focus hints. Keep surrounding context and partially visible images; do not hard-crop to the screen.
2. Preserve original image bytes, captions, nearby text and provenance. Screenshots may help orientation later, but never replace originals with crops or reverse-image searches.
3. Store the full bounded source separately. Send the agent a compact outline/content-block catalogue, then let it read already-captured blocks by ID. These are server-side reads of the snapshot, not actions in the browser. Agent interpretation decides what belongs together.
4. Keep loaded discussion separate from the primary content. Avoid ingesting unrelated feed posts, forms, drafts, scripts and hidden account data. Page text never supplies trusted instructions.
5. Missing content stays explicit. A closed or virtualized transcript may not be in the DOM. Ask the user to open the transcript and capture again; do not promise generic DOM capture can obtain unloaded content. Existing limited X/YouTube extraction can remain as compatibility until generic observations match its useful behavior.

For long transcripts, use bounded section extraction/summaries followed by synthesis, preserving every substantive step/timestamp reference. Keep those section calls inside existing request/token/deadline budgets, and retain the complete source independently. Paging alone does not bound accumulated model context. Design retry/checkpoint behavior before implementation; never drop successful records because a final explanatory response failed.

Possible UI: show the detected source title and a concise missing-content message. A passive focus preview could be useful, but should not create another mandatory decision before capture. Default viewport weighting and how to handle several equally prominent posts remain decisions to test with fixtures, not reasons to add a form.

## Remaining boundaries

- Broader reusable policy profiles or `VAULT.md` management preferences are unnecessary until a concrete need appears. Policy files do not grant permissions.
- Multi-record collection export, resumable/chunked binary transfers, direct individual-asset downloads and a read-only pairing scope are possible follow-ups. Current originals-only export is still a bundle.
- General two-way synchronization, arbitrary external rearrangement repair and built-in encrypted backups remain deferred. No WebDAV or additional required services.
- If host Chromium dependencies are unavailable, extension integration can run in the Playwright image with an isolated test receiver. See validation.

## Suggested skills

- `diagnosing-bugs`: remaining context/transport failures need a reproducible failing case.
- `codebase-design`: design bounded snapshot observations and section processing before implementation.
- `handoff`: keep this project handoff current; explicit user preference overrides the skill's temporary-directory default.
