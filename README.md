# Grünes Gewölbe — GG

GG is a personal archive of ordinary Markdown and image files. Open the browser extension, optionally type instructions, and click **Capture**. Pi interprets the page, chooses a subvault, and preserves the useful content. Use `gg do` or `gg chat` to manage the archive.

The extension and CLI share one controller. The controller owns the vault and provider credentials; Pi works in a fresh isolated worker. Browser captures contain the page already accessible to you, so GG does not need a publicly accessible copy or your login cookies.

## Install

The controller requires Linux, Node.js 22+, pnpm and rootless Podman/crun. Browser and remote CLI clients do not need Podman. See [Perkele deployment](docs/deployment.md) for the service and connecting without a domain.

```sh
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
gg configure --provider openai --model gpt-5.6-luna

gg models openai-codex              # inspect Pi's available model catalog
gg serve
```

Choose **one** provider. `OPENAI_API_KEY` / `OPENROUTER_API_KEY` also work in the controller's environment. No automatic fallback to a paid API occurs. `gg provider PROVIDER MODEL` changes providers explicitly; for a running receiver, issue it through a paired management CLI, or restart after local configuration changes.

Credentials are in the controller's private state directory (`~/.config/gg`, overridden by `GG_STATE_DIR` or `--state-dir`), outside the vault. GG uses Pi's supported OAuth/login and refresh implementation, not another application's credential file. Account access to a model must still be validated by a live sign-in/request. OpenAI distinguishes [subscription sign-in and API-key authentication](https://developers.openai.com/codex/auth/); the account and provider determine actual availability.

## Capture from Chrome / Chromium

1. Load this repository's `extension` directory through `chrome://extensions` → Developer mode → Load unpacked.
2. Click GG on a page, or press **Alt+Shift+G**.
3. On first use, enter the GG address and a code from `gg pair`. Pairing survives server restarts. Codes expire after ten minutes and are single-use.
4. Optionally enter instructions, then **Capture**. No type, destination or image selection is required.

For example: “Save each painting as a separate record, including its attribution.” Without instructions, one page normally produces one coherent record with several relevant images. Unsure classifications go to searchable **Inbox**. Existing source instructions carry forward on recapture unless replaced.

The extension sends visible text, sanitized HTML, captions, an already-visible transcript, and up to 24 image candidates with bytes when obtainable. It excludes form/editor contents, hidden content, scripts, browser storage and cookie stores. Page content itself can contain private information and is sent to your configured model provider. Extraction is best effort. Instructions can ask for captured discussion context; GG does not crawl unloaded replies. For YouTube, open **Show transcript** first; GG does not transcribe audio.

Some cross-origin images need **Allow image hosts (if needed)**. Missing or unsupported images are recorded visibly; available content is still saved. To supply previously missing bytes, grant access and make a new capture from the original page. **Retry unfinished work** reuses the already-received snapshot. After acceptance you can close the tab. Recent captures show progress, retry and cancellation. Usage notifications are optional while the capture screen is open.

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
| `capture-file FILE...`                                 | Process saved browser snapshots locally                                     |
| `search QUERY`, `ask QUESTION`                         | File search or model-assisted read-only retrieval                           |
| `do INSTRUCTIONS`, `chat`                              | Natural-language vault management                                           |
| `list`, `history`, `undo [ID]`, `confirm ID`           | Inspect records, history, restoration and approvals                         |
| `usage`, `resume`                                      | Inspect limits or clear a provider pause after fixing its cause             |
| `override --requests N --tokens N [--minutes N]`       | Explicit temporary extra budget, up to 50 requests / 1M tokens / 60 minutes |
| `provider PROVIDER MODEL`                              | Explicitly switch provider and model                                        |
| `index`, `probe`, `queue`                              | Rebuild Obsidian index, test isolation, inspect legacy decision queue       |

A paired CLI routes archive commands to the server; `--local` selects local operations. Setup, sign-in, device management, `capture-file`, `index`, `probe` and legacy `art`/`painting`/`idea` operations run locally. Legacy commands retain the previous single-image/idea semantics; use `capture` for new behavior. `gg help` lists flags. Commands currently print structured JSON results, including paths, operation IDs and previews.

## Limits

All model work in one controller state directory shares a durable single-job lock and rolling budgets: **50 requests/hour, 300/week, 500,000 tokens/hour, 3,000,000/week**. Warnings start at 80%; new requests pause at the cap. Counters include retries and survive restarts. At most two delayed transient retries; authentication/quota failures pause immediately, and three consecutive provider failures open the circuit breaker. Resume explicitly after correcting the cause. No model calls occur simply to read usage.

Application counters are not subscription percentages. When Codex reports allowance headers, GG displays their last observation; otherwise it says **unavailable**. Usage outside GG is not included in application counters. API calls reserve estimated input plus the requested output cap; the current Pi Codex adapter does not send an output-token cap, so subscription calls reserve the model's full catalog output allowance. Successful reported usage replaces the reservation; failures retain it. Estimates and timeouts limit exposure, not exact spend.

`--config /private/settings.json` supports `provider`, `model`, `vision`, `maxRequests` (10/job), `maxSeconds` (180/job), `maxInputTokens` (64,000 estimated), `maxOutputTokens` (4,096 where supported), and `limits: {hourRequests, weekRequests, hourTokens, weekTokens}`. Run `gg configure --config FILE` to persist settings without an API key. Use the same state directory for every controller command; separate state directories have separate budgets.

Workers have no direct network, provider credentials, vault mounts or runtime socket. Resources are bounded at 2 CPUs, 2 GiB RAM, 256 processes, 150 MB fetched content and 64 MB per image. Image dimensions/fingerprints remain worker-supplied with controller structural validation; this is not a proof against arbitrary compromised-worker behavior.

Receiver uploads are capped at 90 MB, two concurrent uploads, 20 queued jobs and 256 MB retained inputs. Pending jobs resume after restart; interrupted jobs require retry. Cancellation stops unfinished work and retains already-saved records. Paused jobs stay on disk. Failed/partial/cancelled snapshots also remain for recovery; while stopped, remove unneeded job directories to reclaim retained-input space. After a hard crash, verify the writer/receiver is stopped before removing stale vault locks. Also inspect stale state-directory `.reclaim` gates before removing them; never remove locks held by live processes. Inspect incomplete history entries before manual recovery.

## Portable vault

Open the vault directory in Obsidian and start with `GG Index.md`. Records contain native YAML properties, editable summaries, relative image embeds and preserved source links. `.gg-assets.json` tracks originals, `.gg-history` holds restoration data, `.gg-jobs` holds receiver state and `.gg-proposals` holds confirmation previews. `.gg-plans` preserves multi-record plans and per-snapshot completion across retries. Copy the whole vault to retain content and history. Keep credential state separate.

Perkele is the authoritative writer; laptop/desktop copies can be refreshed using existing file-transfer tools. Arbitrary offline edits are not automatically merged. See [deployment and mirror procedure](docs/deployment.md). Encrypted Whatbox backups are a deferred TODO.

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
