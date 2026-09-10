# GG Pi prototype

A real Pi agent runs in a rootless container, reads public sources, prepares images, and submits a proposed capture to a trusted host controller. The controller owns the API key and the file vault. `gg ask` runs Pi with search/read access and returns existing files with relevance explanations.

**Findings:** [NOTES.md](NOTES.md). **Reproducible evidence:** [reports/results.json](reports/results.json). This prototype extends Pi through its SDK; no fork was needed.

This independent package leaves the old Rust/Tauri application and `/home/cem/Gewolbe` intact. It only opens vaults explicitly initialized by this prototype. The persistence tests, runnable examples, and reports are retained for review as requested; that request takes precedence over the Prototype skill's generic in-memory/no-tests defaults. The settings wizard, queue resolution, repository replacement, and hosted GUI are the next implementation phase.

## Run it

Requirements: Linux, rootless Podman with the crun runtime and working user namespaces, Node.js 22 or newer, and pnpm 10.30.3. Tested on Fedora 43 with Podman/crun and Node 25.6.1. Other OS/runtime combinations have not been tested. The container image includes Chromium and image libraries; no browser installation on the host is required.

```sh
cd /home/cem/Sync/Projects/gruenesGewolbe/prototypes/pi-vault
pnpm install --frozen-lockfile
pnpm build:worker
pnpm demo
```

`demo` needs no API key. It runs **real Pi with a deterministic model stub** through a blocked link, an ambiguous image, a successful save, a duplicate, and a search. It prints a fresh disposable vault path and stores a detailed report there. It tests integration, not model judgment.

Create a separate vault for manual captures:

```sh
pnpm gg init --vault .runs/manual Paintings Photography Sculptures Ideas
pnpm gg probe --vault .runs/manual --json
pnpm gg art 'https://commons.wikimedia.org/wiki/File:Le_Chevalier_aux_Fleurs_1894_Georges_Rochegrosse_1859_1938.jpg' \
  --vault .runs/manual --provider openai --model YOUR_MODEL_SLUG
pnpm gg search 'Rochegrosse' --vault .runs/manual
pnpm gg ask 'Find paintings with knights and flowers' \
  --vault .runs/manual --provider openai --model YOUR_MODEL_SLUG
```

Set `OPENAI_API_KEY` or `OPENROUTER_API_KEY` in the **host controller's environment** using your usual secret management. The CLI does not automatically read old GG credentials. Neither the real key nor the vault is mounted into the worker. Use `--provider openrouter --model YOUR_MODEL_SLUG` to select OpenRouter. There is no hardcoded production model catalog.

OpenAI defaults to `openai-responses`; OpenRouter defaults to `openai-completions`. Override with `--api`. The live model configured in the old application rejected Chat Completions and succeeded with Responses. `--config PATH` also accepts a private JSON file:

```json
{
  "provider": "openrouter",
  "model": "YOUR_MODEL_SLUG",
  "api": "openai-completions",
  "vision": true,
  "maxRequests": 10,
  "maxOutputTokens": 4096,
  "maxSeconds": 180
}
```

Use a model with tool calling; art also requires image input. `vision: false` permits text-only idea/ask models and rejects art. Capability compatibility remains the user's configuration; the prototype does not silently change models. Environment API keys override a key in a private config file. Keep credential files outside the repository.

## Commands and behavior

- `init [SUBVAULT...]`: user-controlled setup of an **empty** prototype directory. The agent cannot add or delete destinations.
- `art URL...` / `painting URL...`: capture the selected painting, photograph, or sculpture image into an existing fitting subvault. Preserve image bytes, create a preview, and save brief identification metadata. Ambiguous selection or no fitting destination enters the file queue.
- `idea URL... [--focus 'CONCERN']`: generate a searchable summary with usable instructions where appropriate. Preserve the fetched source text separately, regardless of focus. For YouTube, use existing captions only.
- `--stdin`: add one URL per line to a capture batch. Processing is sequential; failure or a queued input does not stop later entries. Ctrl+C stops the current job and remaining batch entries.
- `search QUERY`: deterministic file search with paths, snippets, and dates; no model call. Other coding agents can use its JSON output directly.
- `ask QUERY`: Pi expands queries, reads candidate records, and returns relevant verified paths. It has no web or write capability. No useful match returns an empty list.
- `queue`: list pending decisions. Selection/resume is deliberately deferred to the clean CLI phase.
- `probe`: verify the actual worker's browser, scratch space, credential separation, blocked private targets, and lack of host mounts/direct networking.
- `--json`: one detailed result object per input. Ordinary output shows progress and a concise outcome.

Examples:

```sh
pnpm gg art 'https://example.org/one' 'https://example.org/two' --vault .runs/manual --model YOUR_MODEL_SLUG
cat links.txt | pnpm gg idea --stdin --focus 'Instructions I can follow' --vault .runs/manual --model YOUR_MODEL_SLUG
pnpm gg queue --vault .runs/manual
```

Inaccessible sources and unavailable transcripts produce `skipped`, no item, and no queue entry. Runtime/provider failures produce `failed` and a nonzero process exit status. Every result identifies its input. A repeated source/item returns `existing`; a conservatively confirmed larger image becomes `upgraded` while earlier copies remain.

## Checks you can inspect or repeat

```sh
pnpm typecheck
pnpm format:check
pnpm test
pnpm test:container
pnpm evaluate:images '/absolute/path/to/example.webp' '/absolute/path/to/large.jpeg'
```

Unit tests cover persistence, source retention, duplicates/upgrades, destination traversal and symlinks, write locking, queue rules, private-address rejection, gateway authentication, fixed model/request limits, read-only ask, and one outcome per input. Container tests use actual Pi/Chromium and a model stub to exercise isolation, batches, provider protocol selection, retrieval, and cancellation followed by a successful entry.

For the live-source evaluation, set `GG_MODEL`, the matching API-key environment variable, and optionally `GG_PROVIDER=openrouter`, then run `pnpm evaluate:live`. This incurs provider usage: nine sequential jobs, each capped at ten model requests and 150 seconds. It writes a fresh vault and detailed report under `.runs/`. No live test runs by default in the automated suite.

## Files and boundaries

```text
prototype-vault/
  .gg-prototype.json
  subvaults/
    Paintings/items/<id>/record.md
                         preview.jpg
                         files/<content-hash>.jpg
    Ideas/items/<id>/record.md
                     source.txt
  queue/<id>.json
  .staging/                  # incomplete new writes
```

Markdown records contain JSON frontmatter (also valid YAML flow syntax), readable summary text, source URL, dates, and relative asset paths. Search scans files; there is no database or embedding service. New items become visible by directory rename. The controller serializes its own writes and rejects a competing process through `.write-lock`. After a hard controller crash, check that no writer is running before manually removing a stale lock from a disposable test vault.

`src/cli.ts` calls reusable operations in `runner.ts` and `vault.ts`. The runner launches a fresh container with no network, host directory, vault, credentials, or runtime-socket mount. A private inherited file descriptor carries bounded traffic to `gateway.ts`. The host forwards model requests to the configured provider and public-site traffic through a DNS/IP-validating proxy. Image decoding and scripts run inside the worker. Chromium's sandbox stays enabled; rootless `SYS_CHROOT` is required by that sandbox on this host.

Defaults per job: 180 seconds, 10 model requests, 4096 output tokens per request; 2 GiB container RAM, 2 CPUs, 256 processes, 150 MB download traffic, 64 MB per fetched file, and 100 million decoded image pixels. These are resource/request bounds, **not an exact currency cap**. API input tokens are billable too. The prototype is a tested isolation experiment, not a production security certification.
