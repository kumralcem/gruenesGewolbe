<p align="center"><img src="assets/brand/logo.svg" width="112" alt="Grünes Gewölbe coral vault logo" /></p>

# Grünes Gewölbe — GG

GG is a personal archive of ordinary Markdown and image files. Open the browser extension, optionally type instructions, and click **Capture**. Pi interprets the page, chooses a subvault, and preserves the useful content. Use `gg do` or `gg chat` to manage the archive.

The extension and CLI share one controller. The controller owns the vault and provider credentials; Pi works in a fresh isolated worker. Browser captures contain the page already accessible to you, so GG does not need a publicly accessible copy or your login cookies.

## Install

GG is a working prototype for a personal archive, with an MIT license. It currently targets a Linux controller and Chromium-based browsers (including Brave); there is no browser-store package or Firefox release. The controller can run on your own Linux machine or server. “Perkele” in the deployment guide is the current author's host, not a service you need access to.

For an **extension-only client**, clone the repository and load `extension/` as described below. No Node.js, pnpm, or Podman is needed on that client. You need access to a configured GG controller and a pairing code.

The controller requires Linux, Node.js 22+, pnpm and rootless Podman/crun. Browser and remote CLI clients do not need Podman. See [Perkele deployment](docs/deployment.md) for the service and connecting without a domain.

```sh
git clone https://github.com/kumralcem/gruenesGewolbe.git
cd gruenesGewolbe
pnpm install --frozen-lockfile
./scripts/install-cli.sh             # ~/.local/bin/gg; keep this checkout installed
pnpm build:worker
gg init --vault "$HOME/Gewolbe"      # requires an empty directory
```

Ensure `~/.local/bin` is on PATH. `init` saves the vault location, so daily commands need no `--vault` or `pnpm` prefix. Default destinations are Paintings, Photography, Sculptures, Ideas and Inbox. Existing archives remain readable; this does not migrate the earlier Rust/Tauri application's archive.

## Choose a provider and sign in

Run sign-in on the controller host, under the same user/state directory as the service:

```sh
# Use Pi's native Codex subscription integration with Luna:
gg login openai-codex
gg configure --provider openai-codex --model gpt-5.6-luna

# Or choose a paid API explicitly:
gg login openai                     # prompts for the API key without echoing it
# gg login openrouter
gg models openai                    # choose a model available to your API account
gg configure --provider openai --model YOUR_MODEL_ID

gg models openai-codex              # inspect Pi's available model catalog
gg serve
```

Choose **one** provider. `OPENAI_API_KEY` / `OPENROUTER_API_KEY` also work in the controller's environment. No automatic fallback to a paid API occurs. `gg provider PROVIDER MODEL` changes providers explicitly; for a running receiver, issue it through a paired management CLI, or restart after local configuration changes.

Credentials are in the controller's private state directory (`~/.config/gg`, overridden by `GG_STATE_DIR` or `--state-dir`), outside the vault. GG uses Pi's supported OAuth/login and refresh implementation, not another application's credential file. Account access to a model must still be validated by a live sign-in/request. OpenAI distinguishes [subscription sign-in and API-key authentication](https://developers.openai.com/codex/auth/); the account and provider determine actual availability.

## Capture from Chrome / Chromium

1. Load this repository's `extension` directory through `chrome://extensions` → Developer mode → Load unpacked.
2. Click GG on a page, or press **Alt+Shift+G**.
3. On first use, open **Settings & recent captures**, then **Connect to GG**. Enter the GG address and a code from `gg pair` on the controller. Pairing survives server restarts. Codes expire after ten minutes and are single-use.
4. Optionally enter instructions, then press **Enter** or click **Capture**. **Shift+Enter** inserts a new line. No type, destination or image selection is required.

For a remote controller without a public HTTPS endpoint, keep an SSH tunnel open as shown in the [connection guide](docs/deployment.md#connect-without-buying-a-domain); the extension uses `http://127.0.0.1:48123`.

For example: “Save each painting as a separate record, including its attribution.” Without instructions, one page normally produces one coherent record with several relevant images. Unsure classifications go to searchable **Inbox**. Existing source instructions carry forward on recapture unless replaced.

For a text instruction set, try: “Save the five tips as actionable instructions, without images. Create SoloDev under Ideas and save it there.” Capture can create explicitly requested destinations for new records; it cannot rename, move or delete existing records. Recapture keeps an existing record in its current folder; move it with `gg do` if needed. Use `gg do` for those actions. When the agent chooses a text-only idea record, irrelevant image failures do not make it partial. Missing relevant images and truncated source content still produce a partial result. The browser may still collect candidate images before the agent decides what is relevant.

Appearance defaults to **Dark**. Choose **Dark**, **Light** or **System** on the settings page; the choice applies to both settings and the popup, and System follows your OS/browser preference.

The extension sends visible text, sanitized HTML, captions, an already-visible transcript, and up to 24 image candidates with bytes when obtainable. It excludes form/editor contents, hidden content, scripts, browser storage and cookie stores. Page content itself can contain private information and is sent to your configured model provider. Extraction is best effort. Instructions can ask for captured discussion context; GG does not crawl unloaded replies. For YouTube, open **Show transcript** first; GG does not transcribe audio.

Cross-origin restrictions or network failures can prevent image downloads. GG records missing images when they are relevant to the saved record; available content is still saved. To supply previously missing bytes, make a new capture from the original page. **Retry unfinished work** reuses the already-received snapshot and cannot fetch missing browser images. After acceptance you can close the popup. The settings page shows recent captures, retry and cancellation. Usage notifications are optional while that page is open.

Browser source URL aliases (YouTube video links and X/Twitter post links) reuse the submitted snapshot. Model-facing page/HTML reads are bounded and paginated; complete supplied source text remains preserved. Transcript captures do not automatically attach thumbnail previews. The configured model input limit still applies to unusually large or repeated reads.

## Shared capture instructions

`CAPTURE.md` in the vault root controls the default level of detail and writing style for new captures and recaptures. GG creates it with useful defaults if it is missing, without replacing an existing file. For lists, those defaults ask for every substantive tip, an explanation of each, and concrete actions or examples. Tutorials retain steps, prerequisites and caveats; essays retain their argument and supporting points.

Edit the file directly, or open the extension's **Settings & recent captures → Capture instructions**, edit, and click **Save instructions**. Both edit the same server-side file, shared by all your devices. No service restart is needed. The editor detects stale copies rather than overwriting someone else's changes; reload explicitly after a conflict. Settings edits appear in GG history and can be undone. The limit is 16,000 UTF-8 bytes; an empty file disables the shared defaults.

Instructions for an individual capture override these defaults. Saved source instructions still carry forward on recapture when no new instructions are supplied. Changes do not rewrite existing records automatically, and recapture continues to preserve human edits. Source pages cannot edit `CAPTURE.md`; the worker only receives a read-only snapshot for its job. Paired browser devices may edit these capture preferences, without gaining general management permissions. The file expresses content/style preferences, not additional tool permissions or authorization to rearrange the vault.

## Nested folders and automatic routing

GG discovers existing folders under your vault's `subvaults/` directory on each operation. For example:

```sh
mkdir -p "$HOME/Gewolbe/subvaults/Photography/Historic"
```

The next capture can automatically choose `Photography/Historic`. No registration or restart is needed. You can also use `gg do 'Create Historic inside Photography'`; the agent uses the full relative path. Parent folders may still receive captures themselves. GG creates `items/` inside a manually created destination when it first saves a record there.

Hidden folders, symlinks and the reserved `items/` trees are excluded from discovery. Paths support up to 16 levels, 100 characters per folder name and 500 characters overall. Existing vaults keep their layout; no reinitialization is needed. After updating this checkout, rebuild the worker image and restart a running controller once to load the new code. Undo of folder creation/renaming recorded before this update requires manual recovery because the old history omitted directories; other file history remains usable.

GG can inspect existing GG records to help route captures. Loose files are not automatically imported, and manually moving records or individual assets is not reconciled yet. Use `gg do` for those operations so metadata and history stay consistent.

## Updates, management and undo

Recapture updates records matched by source URL and stable capture key. Each update includes a dated note. Human-edited fields and summaries win on conflict; proposed generated values remain in `.gg-baseline.json`. Earlier media remains preserved. Clearly matched split records update individually; ambiguous matches are rejected without overwriting originals. Successful records survive later failures in a multi-record job.

```sh
gg do 'Move the email triage note to Ideas and tag it customer-support'
gg do 'Create a subvault called Architecture'
gg chat
gg history
gg undo OPERATION_ID                # or BATCH_ID; no ID means latest operation
```

Moves, edits, and creating/renaming subvaults execute on your instruction. Delete and merge return a concrete preview and `proposalId`; only `gg confirm PROPOSAL_ID` (or `/confirm` in chat) executes it. Changed records invalidate a preview. Deletion uses `.gg-trash`. Undo rejects later edits it would overwrite and supports complete batches. History is local and grows with changes; it is not an off-machine backup. Multi-file changes use a journal and atomic individual writes, not database transactions or fsync-level crash guarantees.

## Commands

| Command                                                | Purpose                                                                     |
| ------------------------------------------------------ | --------------------------------------------------------------------------- |
| `init`, `configure`                                    | Create a vault and save controller settings                                 |
| `login`, `logout`, `models`                            | Pi provider credentials and model catalog                                   |
| `serve`                                                | Run the loopback receiver and background capture queue                      |
| `pair --scope capture\|manage`, `devices`, `revoke ID` | Pair and revoke individual devices                                          |
| `connect URL`                                          | Pair a remote management CLI; prompts for a code                            |
| `capture URL... [--instructions TEXT] [--stdin]`       | Public URL capture; browser capture handles signed-in pages                 |
| `import PATH... [--instructions TEXT]` | Import local JPEG, PNG and WebP files or directories, locally or to the paired server |
| `capture-file FILE...`                                 | Process saved browser snapshots locally                                     |
| `search QUERY`, `ask QUESTION`                         | File search or model-assisted read-only retrieval                           |
| `do INSTRUCTIONS`, `chat`                              | Natural-language vault management                                           |
| `list`, `history`, `undo [ID]`, `confirm ID`           | Inspect records, history, restoration and approvals                         |
| `usage`, `resume`                                      | Inspect limits or clear a provider pause after fixing its cause             |
| `override --requests N --tokens N [--minutes N]`       | Explicit temporary extra budget, up to 50 requests / 1M tokens / 60 minutes |
| `provider PROVIDER MODEL`                              | Explicitly switch provider and model                                        |
| `index`, `probe`, `queue`                              | Rebuild Obsidian index, test isolation, inspect legacy decision queue       |

A paired CLI routes archive commands to the server; `--local` selects local operations. Setup, sign-in, device management, `capture-file`, `index`, `probe` and legacy `art`/`painting`/`idea` operations run locally. Legacy commands retain the previous single-image/idea semantics; use `capture` for new behavior. `gg help` lists flags. Agent commands print concise answers, saved paths, undo commands, confirmation previews and warnings. Add `--verbose` or `--json` for the full diagnostic result. Interactive terminals show elapsed time while waiting; `--json` suppresses that indicator for scripts. Other administrative commands retain structured output. The daemon keeps the controller running but starts a fresh isolated worker for each agent job; model requests and worker startup still take time.

## Importing local images

On the machine containing your images, install the CLI and use `gg connect SERVER_URL` to pair it with your server. Then run:

```sh
gg import ~/Pictures/Collection --instructions "Organize these under Art"
gg import photo.jpg scan.png --instructions "Create Ideas/References and save these there"
```

Directories are scanned recursively. JPEG, PNG and WebP files up to 20 MB each are supported; hidden directory entries, symbolic links and unrelated files inside directories are skipped. Explicitly supplied unsupported files produce an error. Originals stay on your machine and are copied unchanged into records; the server receives the filename, image bytes and instructions, not your full local path. Embedded image metadata remains part of the original file.

Each image defaults to one record. The agent inspects it and chooses an existing destination, using `CAPTURE.md` plus your instructions. This consumes normal model usage. Images upload and process one at a time through the server's capture queue and shared limits. The command prints each filename and waits for its result before sending the next image. A failure or usage pause stops the batch.

Run the same command again to resume: completed images are skipped by exact file contents, even after renaming. Unfinished uploads/jobs reconnect or retry. Changing instructions does not rewrite already imported images; use `gg do` to change those records. Ctrl+C stops the CLI; an already accepted server job can finish. If the CLI stops waiting after 15 minutes, rerun it to reconnect. Without a paired server, import uses the configured local vault and the same worker/usage safeguards (`--local` explicitly selects this mode).

This does not import arbitrary documents, RAW/HEIC files, or reconcile files manually dropped into the vault. `capture-file` remains the browser-snapshot JSON command.

## Limits

All model work in one controller state directory shares a durable single-job lock and rolling budgets: **50 requests/hour, 300/week, 500,000 tokens/hour, 3,000,000/week**. Warnings start at 80%; new requests pause at the cap. Counters include retries and survive restarts. At most two delayed transient retries; authentication/quota failures pause immediately, and three consecutive provider failures open the circuit breaker. Resume explicitly after correcting the cause. No model calls occur simply to read usage.

Application counters are not subscription percentages. When Codex reports allowance headers, GG displays their last observation; otherwise it says **unavailable**. Usage outside GG is not included in application counters. API calls reserve estimated input plus the requested output cap; the current Pi Codex adapter does not send an output-token cap, so subscription calls reserve the model's full catalog output allowance. Successful reported usage replaces the reservation; failures retain it. Estimates and timeouts limit exposure, not exact spend.

`--config /private/settings.json` supports `provider`, `model`, `vision`, `maxRequests` (10/job), `maxSeconds` (180/job), `maxInputTokens` (64,000 estimated), `maxOutputTokens` (4,096 where supported), and `limits: {hourRequests, weekRequests, hourTokens, weekTokens}`. Run `gg configure --config FILE` to persist settings without an API key. Use the same state directory for every controller command; separate state directories have separate budgets.

Workers have no direct network, provider credentials, vault mounts or runtime socket. Resources are bounded at 2 CPUs, 2 GiB RAM, 256 processes, 150 MB fetched content and 64 MB per image. Image dimensions/fingerprints remain worker-supplied with controller structural validation; this is not a proof against arbitrary compromised-worker behavior.

Receiver uploads are capped at 90 MB, two concurrent uploads, 20 queued jobs and 256 MB retained inputs. Pending jobs resume after restart; interrupted jobs require retry. Cancellation stops unfinished work and retains already-saved records. Paused jobs stay on disk. Failed/partial/cancelled snapshots also remain for recovery; while stopped, remove unneeded job directories to reclaim retained-input space. After a hard crash, verify the writer/receiver is stopped before removing stale vault locks. Also inspect stale state-directory `.reclaim` gates before removing them; never remove locks held by live processes. Inspect incomplete history entries before manual recovery.

## Portable vault

Open the vault directory in Obsidian and start with `GG Index.md`. Records contain native YAML properties, editable summaries, relative image embeds and preserved source links. `.gg-assets.json` tracks originals, `.gg-history` holds restoration data, `.gg-jobs` holds receiver state and `.gg-proposals` holds confirmation previews. `.gg-plans` preserves multi-record plans and per-snapshot completion across retries. Copy the whole vault to retain content and history. Keep credential state separate.

Your controller is the authoritative writer; laptop/desktop copies can be refreshed using existing file-transfer tools. Arbitrary offline edits are not automatically merged. See [deployment and mirror procedure](docs/deployment.md). Encrypted Whatbox backups are a deferred TODO.

## Validation

```sh
pnpm typecheck
pnpm format:check
pnpm test
pnpm test:worker                     # real Pi + stubbed provider; no isolation claim
pnpm test:container                  # actual Podman boundary
pnpm test:extension                  # Chromium + cookie-protected fixture
pnpm demo                           # deterministic container demo, no provider key
```

See [validation](docs/validation.md) for results and environmental blockers. Live provider sign-in, arbitrary website behavior and public deployment require separate checks; fixture success does not establish them.

## Project identity

The coral vault mark combines phthalo green, coral and warm gold. See the [three logo options](assets/brand/options.svg) and [editable SVG sources](assets/brand/README.md). Extension icons are included; no local build is needed to load the extension.

### Updating the unpacked extension

After `git pull` on `master`, open `chrome://extensions` (or your Chromium browser’s extensions page) and click **Reload** on GG Capture. Close old GG tabs. Version **0.4.1** shows the coral icon and a compact capture popup when you click the toolbar icon or use Alt+Shift+G. **Settings & recent captures** opens the full page for pairing and history. If the old GG letters or idea/image selector remain, check that the loaded extension directory is the `extension/` folder of this checkout, rather than another clone.
