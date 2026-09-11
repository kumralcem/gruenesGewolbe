# Clean GG restart, Chrome capture, and Obsidian records

Status: ready-for-human (implemented, tested, and reviewed)

## Authorized scope

The user approved removing irrelevant old code on `codex/pi-vault-prototype`, leaving other branches and the live archive unchanged. Promote the working Pi prototype to the root as the new implementation. Retain its useful image-format/thumbnail handling and tests. Remove the Rust/Tauri application, its obsolete tests/scripts, and superseded implementation documentation.

Add a Chrome/Chromium extension (the user's selected first browser). An explicit capture uses the user's already-open authenticated page to collect relevant text/HTML and one selected image's bytes. Send that material to the agent for processing; browser credentials stay in the browser. The browser UI uses the CLI's controller/worker operations and automatic filing rules. Processing should survive closing its capture tab.

Make the per-item `record.md` usable directly in Obsidian with native properties, tags, readable summaries, visible dates, and relative links/embeds. Preserve the shallow file archive without a database. Existing user edits must survive image upgrades. Apply this to new GG captures; do not migrate or alter `/home/cem/Gewolbe`.

## Carried-forward behavior

One visual capture means the exact selected image, excluding discussion and alternate viewpoints. Only the user creates/deletes vaults/subvaults. Agents choose existing destinations or queue uncertainty. Keep originals alongside same-image improvements. Ideas preserve useful instructions, searchable summaries, source text, and URLs. Missing transcripts are skipped without audio transcription. Direct URL batches report each failure and continue. `search` is deterministic; `ask` retrieves files without modifying them. OpenAI/OpenRouter model slugs remain configurable. Provider keys and the vault remain outside the agent container.

## Acceptance

- The root package runs and old application code is absent on this branch; other branch refs and live archive remain unchanged.
- Chromium integration proves capture of a cookie-protected page and exact image without exporting cookie/form/script/browser-storage data.
- A real Pi container processes an injected snapshot without re-fetching its private source.
- Receiver pairing, web-origin rejection, idempotency, interruption/retry, and malformed-job isolation are exercised.
- A copied vault has working Obsidian links; YAML records remain searchable after user edits, and media upgrades preserve notes/properties.
- Root setup instructions, format documentation, evidence, typechecking, formatting, unit tests, container tests, and browser tests are current.

## Deferred

Firefox, hosted deployment, bulk live-archive conversion, a settings wizard, and decision-queue approval UI are not part of this request. Browser job retry is included. No live X authentication or OpenRouter compatibility claim without direct verification.
