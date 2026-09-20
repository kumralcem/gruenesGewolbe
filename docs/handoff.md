# Next-session handoff

Updated 2026-09-20. Current implementation: [README](../README.md), [architecture](adr/0051-current-architecture.md), [validation](validation.md). Historical interviews/reviews are recoverable from Git; do not treat their superseded workflows as current requirements.

## Scope and priority

The user authorized documentation cleanup and an import-quality check. **Per-subvault policy, the browser-capture redesign and new upload/download interfaces are proposals only: do not implement them without a subsequent request.** Reliability should come before another unattended collection run.

## Import status and transport failure

The latest collection run, `gg-import-paintings-verified`, failed at item 10 with an unhandled `write EPIPE`. The stack points to `socket.end(...)` in the proxy CONNECT catch handler in `src/gateway.ts` (around line 766 at `84d94c6`). Nine images were saved in total. This is observed evidence, not a deterministic reproduction or a proven explanation for the earlier `ECONNRESET` crash.

```sh
systemctl --user status gg-import-paintings-verified --no-pager
tail -n 40 ~/.local/state/gg/import-paintings.log
gg usage --local
gg usage history --local
```

Source directory is `~/gg-import/Paintings`, 132 images including one 31.2 MB original. Do not delete staging files. The normal `gg.service` is separate. Check active jobs before restarting anything. The import was not restarted after this crash.

A collection-specific grant was authorized after commit/push: 1,500 requests, 50M tokens, 12 hours from creation, restricted to the exact 132 image hashes. Find its ID/remaining allowance through `gg usage`; never renew it automatically. The failed command used `--local --grant ID --continue-on-error`, requesting suitable children of Art and factual title/artist/date attribution. Re-running skips stored records; a process crash defeats per-image continuation. Provider pauses and per-job limits remain in force.

First seven records were manually reviewed and normalized in undo batch `19d5e36b-114d-4952-9fc8-0e468c0a100c`. The two subsequent saves selected Art/Paintings and explicitly marked attribution provisional after failed research. Their actual images and records should be checked when evaluating quality; a successful save is not proof of verified attribution. See validation for evidence limitations.

Next investigation: reproduce closed-client/proxy error handling, contain transport errors to one job, and test receiver recovery and batch continuation. Do not merely increase usage caps or repeatedly restart the live batch.

## Per-subvault Markdown guidance — not implemented

User wants domain-specific capture preferences close to their destination: painting identification rules should apply under Art/Paintings, not clutter article capture. Today only root `CAPTURE.md` is editable; some local-artwork guidance is hardcoded in the worker prompt.

Options:

1. **Exact-destination file only:** root defaults plus `subvaults/Art/Paintings/CAPTURE.md`. Simple and predictable, but repeats shared Art guidance across siblings.
2. **Hierarchical CAPTURE.md files (recommended):** root → Art → Art/Paintings, with child guidance refining parent defaults. Art can request attribution/provenance; Paintings can emphasize creation date, medium and title variants; Ideas can emphasize actionable steps. Individual capture instructions remain highest-priority preferences. Absent files preserve inherited defaults. A parent’s domain guidance should be broad enough for all its children.
3. **Named profiles with explicit assignment:** destinations reference reusable Markdown profiles such as artwork or tutorial. Useful when unrelated folders share behavior, but adds configuration and indirection; probably premature for this vault.

Example layout (proposal, not a required migration):

```text
CAPTURE.md
subvaults/Art/CAPTURE.md
subvaults/Art/Paintings/CAPTURE.md
subvaults/Ideas/CAPTURE.md
```

Recommended flow: show the folder catalogue and brief descriptions, let the agent propose a destination, then load only that destination's policy chain before interpreting/saving. Do not load every folder's full instructions into every job. If the destination changes, recompute the effective policy; split captures may need a policy per record. A future settings view should show the effective instructions and their source files.

Keep the following decisions explicit without turning the UI into a questionnaire:

- Default precedence: global safety/tool constraints remain enforced in code; within preferences, individual instructions > leaf > ancestors > root. Metadata truthfulness/evidence checks cannot be disabled by a policy file.
- Separate routing hints from detailed capture rules to avoid needing a destination before understanding how to choose it. Avoid mandatory manual folder selection.
- Bound combined policy size and snapshot revisions per job. Policy changes affect future captures/explicit recaptures, not silent archive rewrites. Retry behavior must not change completed records or split plans.
- Restrict reads/writes to approved policy paths, reject symlinks, preserve existing content, and use revision checks/history. Captured web content cannot create or edit trusted instructions.
- Folder moves naturally carry local policy files; parent inheritance then changes for future captures. Document this consequence.
- `CAPTURE.md` should govern capture. A broader `VAULT.md` covering retrieval/management could come later; do not silently grant new agent permissions through prose.

## Generic browser capture — not implemented

Keep the extension interaction minimal: click/shortcut, optional instructions, submit. The user rejected obligatory region/type/image selection and challenged domain-specific X/YouTube extractors: agentic interpretation was a reason for this architecture.

Preferred direction is generic browser observations: visible text, lightweight structure/layout, links, captions and original image bytes. The viewport is a focus hint, not a hard crop; retain surrounding context and partially visible media. Let the agent identify coherent content and request more reads when needed. Screenshots may orient it but must not replace original images with crops.

X normally means the particular post and its media; YouTube normally means transcript plus title/channel/URL. Full transcripts may require opening UI/scrolling; virtualized off-screen content may not exist in the DOM. Additional reads/actions must occur in the signed-in browser, without exporting cookies. This capability and tab-lifetime/action boundaries still need design; do not promise a viewport snapshot contains everything.

Keep full source outside model context. For long transcripts, consider bounded section summaries followed by synthesis preserving steps/timestamp references. Paging alone does not help when every previous page remains in the conversation. Avoid a required extra completion solely to acknowledge a completed save. Existing extraction already strips scripts and attempts post isolation; large HTML is not established as the cause of the X crash.

## Upload/download options — not implemented

1. **Archive page (recommended interface):** drag-and-drop or multi-file picker, instructions, progress/resume and job status. Search/browse records, download the original image or a bundle containing Markdown and preserved files. Keep the capture popup small.
2. **CLI transfer:** existing paired `gg import LOCAL_PATH` already avoids rsync. Proposed `gg download RECORD_ID --to PATH` / `gg export ...` provide originals, record bundles or selected collections without SSH. Downloads need no model call.
3. **Both over shared APIs (recommended architecture):** reuse pairing, limits, streaming and resumability. Resolve verified record/file IDs rather than arbitrary filesystem paths; handle filename collisions and traversal/symlinks. Consider read/download permission separate from management. Stream big transfers instead of buffering a whole collection.
4. **Optional WebDAV/SFTP:** familiar file-manager access, but copying bytes is not indexed import and arbitrary rearrangement remains a separate issue. Avoid another required daemon.

Decide whether a download means originals only, portable record bundles, or a full vault/history snapshot. General two-way synchronization is not part of this feature. Encrypted Whatbox backups remain a deferred TODO.

## Suggested skills

- `diagnosing-bugs`: build a repeatable proxy-disconnect reproduction before fixing the transport failure.
- `codebase-design`: design policy resolution or the browser-observation/upload interfaces once requested.
- `grill-me`/`grilling`: only if the user requests more interviewing; bundle questions and prefer sensible defaults.
- `handoff`: keep this project handoff current. The user's explicit project-file preference overrides the skill's temporary-directory default.
