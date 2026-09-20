# Next-session handoff

Updated 2026-09-20. Current behavior and setup: [README](../README.md), [architecture](adr/0051-current-architecture.md), [validation](validation.md). Keep this file in the project; historical interviews/reviews remain in Git.

## Authorization and decisions

The user authorized fixing the import and implementing inherited folder instructions and upload/download interfaces. They explicitly rejected model-controlled browser clicks/scrolling for the extension. The larger browser capture redesign below is still design work, not permission for browser interaction.

Implemented in this iteration:

- CONNECT sockets get error handling before validation/DNS; rejected broken pipes no longer terminate the controller. Regression reproduces the original EPIPE and checks the next request. This does not prove every possible transport failure is fixed.
- Repeated identical image previews are removed from model context; public link/image candidates are bounded; local image research gets at most four browse/fetch reads and switches to save-only tools at 70% of the conservative input bound. Evidence and conversation history are retained. Completed captures no longer need another paid completion to announce success. Full originals remain intact. These bounds are not general long-document compaction.
- Root → parent → child `CAPTURE.md` inheritance, settings editor/effective view, bounded files, revision checks/history and saved policy chains for retries. Nonempty folder instructions must be read before saving. Source pages cannot edit policy or expand capabilities.
- Settings-page image picker/drop import through the existing durable queue. Archive search and portable `.tar.gz` downloads, also `gg download ID --to FILE.tar.gz [--originals-only]`. Management pairing required for archive access; capture-only tokens remain sufficient for uploads. Bundles exclude history/internal state; current bundle limit 128 MB. Browser buffers one bundle, CLI/server stream it. No new daemon or dependency.
- Art and Art/Paintings guidance files were added to the live vault without overwriting existing policy files.
- Uncertain creator/year values cannot be copied verbatim into confident artist/date tags (normalized punctuation/case). This is a narrow consistency check, not factual authentication.

## Collection run

Source: `~/gg-import/Paintings`, 132 images including one 31.2 MB original. Preserve staging files. The collection grant is restricted to those exact hashes: 1,500 requests, 50M tokens, 12 hours from creation. Check remaining allowance/expiry with `gg usage`; do not automatically extend it or bypass a provider pause.

```sh
systemctl --user status gg-import-paintings-verified --no-pager
tail -n 40 ~/.local/state/gg/import-paintings.log
gg usage --local
gg usage history --local
```

The original run stopped at item 10 with EPIPE. After the socket fix it saved item 10 and continued through item 19; items 11 and 18 hit the model-input bound. The run was stopped gracefully at item 20 to deploy the context/policy changes together. An initial retry still overflowed on item 11. A diagnostic retry then saved that image but reached 62,527/64,000 estimated input tokens: little room remained after research. A further regression now exercises switching to finalization tools early, preserving evidence and room to save. The finalized build `fd489b6` is deployed and both services were restarted. Latest audit: 20 of 132 collection images saved, all 20 originals matched staging byte-for-byte; processing resumed at item 21. Check the live unit/log for subsequent progress; do not infer completion from this snapshot. The normal `gg.service` is separate. Re-running skips existing records by source hash and retries missing ones.

Quality: the first seven records had a separate manual review (undo batch `19d5e36b-114d-4952-9fc8-0e468c0a100c`). The next two were visually consistent with their originals and routed to Art/Paintings; their filename-derived artist/title/date claims remained explicitly uncertain after research failed. All first nine originals matched staging byte-for-byte. A later sample (the filename-labelled Daniel painting) used an overconfident gender description. It was corrected to the visible robed, bound figure through undoable edit `7801fb30-55e2-4bec-af07-9688836cc85a`; Art guidance now explicitly avoids guessed figure identities. A positive new example is Jael and Sisera: Luna saved source-supported title, Artemisia Gentileschi attribution and 1620 date from the [Museum of Fine Arts, Budapest](https://www.mfab.hu/artworks/jael-and-sisera/). A separate check confirmed the catalogue fields and matched its reference image to the imported original. Reliable automatic artwork verification across the collection remains unproven; do not equate successful preservation with verified attribution. Inspect a fresh sample after reliability changes, without silently rewriting uncertain claims as facts.

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
- General two-way synchronization, arbitrary external rearrangement repair and encrypted Whatbox backups remain deferred. No WebDAV or additional required services.
- Host Chromium lacks OS libraries; extension integration runs in the existing Playwright image with an isolated test receiver. See validation.

## Suggested skills

- `diagnosing-bugs`: remaining context/transport failures need a reproducible failing case.
- `codebase-design`: design bounded snapshot observations and section processing before implementation.
- `handoff`: keep this project handoff current; explicit user preference overrides the skill's temporary-directory default.
