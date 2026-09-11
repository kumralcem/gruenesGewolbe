# Grünes Gewölbe — GG

A personal archive built around Pi. Save a page or one image, let the agent describe it and choose an existing subvault, then find it again with `search` or `ask`. The archive is ordinary files and Obsidian-compatible Markdown. There is no database.

This branch replaces the previous Rust/Tauri application with the Pi CLI, isolated worker, and Chrome/Chromium capture extension. The earlier code remains in Git history. The existing archive at `/home/cem/Gewolbe` has not been modified or migrated.

## Start

Requires Linux, rootless Podman with crun and working user namespaces, Node.js 22+, and pnpm 10.30.3. Tested on Fedora 43 and Node 25.6.1. Other operating systems and runtimes are untested.

```sh
pnpm install --frozen-lockfile
pnpm build:worker
pnpm gg init --vault .runs/my-vault Paintings Photography Sculptures Ideas
```

`init` requires an empty directory. Only the user defines destinations; the agent cannot create or delete them. Use a new vault while trying the replacement.

Set `OPENAI_API_KEY` or `OPENROUTER_API_KEY` in the controller's environment, then start the receiver:

```sh
pnpm gg serve --vault .runs/my-vault --provider openai --model YOUR_MODEL_SLUG
# Or: --provider openrouter --model YOUR_MODEL_SLUG
```

The receiver prints its local address and a pairing token. Keep the terminal running. Each receiver start has a new token. No model name is hardcoded; use a model supporting tool calls, and image input for art.

## Capture from Chrome / Chromium

1. Open `chrome://extensions`, enable **Developer mode**, choose **Load unpacked**, and select this repository's `extension` directory.
2. Pin **GG Capture**. On the page you want to save, click GG (or press Alt+Shift+G).
3. In **Connect to GG**, enter the receiver address and pairing token printed by the CLI.
4. Choose **An idea or instructions**, or **One image** and select the exact image. Add an optional summary focus, then **Send to my vault**.

GG reads the page already open in your browser. Its page/image requests can use your browser session; the agent receives captured content and image bytes. It does not receive your cookie store, form contents, scripts, or browser storage. Captured content is sent to your configured model provider for processing.

The extension initially has access only to its local receiver and the tab you explicitly capture. A cross-origin image may need the **Allow image host and retry** button. This requests permission for that image's host. Some sites still block downloading an image even when it is visible; GG reports the failure.

Select text before clicking GG to capture only that text. For YouTube, open **Show transcript** first: the extension reads the transcript currently rendered on the page. It does not generate transcripts or download audio. Without a usable transcript, the agent skips the video. Page extraction is best effort; authenticated fixtures are tested, but the supplied live X posts have not yet been retested through the extension.

Once accepted, a capture continues when its tab closes. **Recent captures** shows results and provides **Retry** for failed/interrupted jobs. Pending jobs resume after receiver restart; interrupted jobs need an explicit retry. Uncertain image selection or destination remains a separate decision queue (`gg queue`). Resolving that decision queue through a UI is still future work.

## Open in Obsidian

In Obsidian, choose **Open folder as vault** and select the directory passed to `--vault`. Open **GG Index.md** to browse saved items. No plugin or conversion step is required for new captures.

Each `record.md` has native YAML properties, a title alias, tags, visible dates, a readable summary, a source link, and relative image/source links. Full image files are embedded directly; bounded `preview.jpg` files also exist for future gallery use. You can edit the summary and add properties or notes. GG reads edited summaries and preserves additional properties and notes during later image upgrades.

```text
my-vault/
  GG Index.md
  .gg-vault.json
  subvaults/
    Paintings/items/<id>/
      record.md
      preview.jpg
      files/<content-hash>.jpg
      .gg-assets.json
    Ideas/items/<id>/
      record.md
      source.md
      .gg-assets.json
  queue/<id>.json
  .gg-jobs/<id>/             # receiver input and processing state
  .staging/                 # incomplete new item writes
```

The layout stays shallow, with one folder per saved item. Copy the entire vault to retain all links and media. `GG Index.md` is generated and refreshed after captures; use a separate note for your own index. `pnpm gg index --vault PATH` rebuilds it. Renaming GG-managed item directories or removing required record properties can prevent GG from recognizing them.

Earlier Pi prototype vaults remain readable. New writes use this format; there is no bulk conversion or importer for the removed application's archive.

## CLI

```sh
pnpm gg art 'https://example.org/image-page' --vault .runs/my-vault --model YOUR_MODEL_SLUG
pnpm gg idea 'https://example.org/article' --focus 'Instructions for managing customer email' --vault .runs/my-vault --model YOUR_MODEL_SLUG
pnpm gg search 'customer email' --vault .runs/my-vault
pnpm gg ask 'Find ways to organize customer correspondence' --vault .runs/my-vault --model YOUR_MODEL_SLUG
pnpm gg queue --vault .runs/my-vault
```

- `art` (also `painting`) preserves one selected image, brief identification metadata, and any conservatively confirmed better copy. Surrounding discussion is not saved.
- `idea` preserves source text and a searchable summary. Instructions retain actionable steps and conditions. Replies are excluded by default.
- Both accept several URLs or `--stdin` with one URL per line. Each result is independent; failures are reported and processing continues.
- `search` scans local files without a model. `ask` uses Pi to search and read relevant files and return verified paths; it cannot change the vault or browse the web.
- `capture-file FILE...` processes the same JSON snapshot contract used by the extension.
- `probe` checks worker isolation. `--json` prints detailed per-input results.

Direct URL capture uses public access. Inaccessible sources and unavailable transcripts are skipped without saving an item. Provider/runtime failures return a nonzero exit status. Duplicates return the existing path; a confirmed larger copy upgrades the primary image while keeping earlier files.

OpenAI uses Responses by default; OpenRouter uses Chat Completions. `--api` overrides this. `--config /private/path/settings.json` accepts:

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

API keys in the environment take precedence over a private config's `apiKey`. Keep credentials outside the repository and vault. Set `vision: false` for text-only models; art jobs will be rejected. A CLI settings wizard is still planned.

## Implementation and limits

`src/cli.ts` and `src/receiver.ts` share `runner.ts` and `vault.ts`. Pi runs in a fresh rootless container. The controller holds the provider key and archive; the worker has neither mounted. A private inherited descriptor connects the worker to its constrained gateway. Public research passes through an address-validating proxy. Browser snapshots travel through that same job channel. The receiver binds only `127.0.0.1`, requires a random pairing token, and rejects ordinary website origins.

Captures default to 180 seconds, 10 model requests, and 4096 output tokens per request. The worker has 2 GiB RAM, 2 CPUs, 256 processes, 150 MB download traffic, 64 MB per file, and a 100-million-pixel decode limit. These bound resource use rather than guarantee an exact monetary cap.

Receiver inputs persist in `.gg-jobs` until saved, already present, upgraded, or skipped. Failed/queued input remains for recovery. Uploads are capped at 90 MB, pending jobs at 20, and retained input at 256 MB. After a hard crash, check that the receiver/writer is stopped before removing a stale `.receiver-lock` or `.write-lock`. Publication uses atomic renames, without fsync-level crash guarantees.

Image identity checks are intentionally conservative. Worker-prepared dimensions/fingerprints are structurally validated by the controller, without independent trusted decoding. Model judgment, malicious-worker resistance, and arbitrary website support still need broader validation. A hosted GUI, decision-queue editing, and archive migration are outside this change.

## Checks and evidence

```sh
pnpm typecheck
pnpm format:check
pnpm test
pnpm test:container
pnpm exec playwright install chromium  # only needed for the extension test
pnpm test:extension
pnpm demo                             # real Pi, deterministic model stub; no key needed
```

The checks cover 23 unit tests, 9 container tests, and a real Chromium extension test. The browser test uses a cookie-protected fixture, verifies exact image bytes and excluded form/script/hidden content, and writes `.runs/extension-evidence/capture.png`. A separate container test sends the snapshot through real Pi without re-fetching the private source. Vault tests copy the archive and verify portable links and search after external Markdown edits.

[Current validation](docs/validation.md) records results and manual checks. [Earlier prototype findings](docs/prototype-results.md) retain the live-source and large JPEG/WebP evidence, including public X/YouTube failures and the untested live OpenRouter path.
