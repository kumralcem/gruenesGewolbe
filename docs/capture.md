# Capturing and organizing

[Home](../README.md) · [Setup](getting-started.md) · [File transfers](files.md)

## Capture from Chrome / Chromium

1. Load this repository's `extension` directory through `chrome://extensions` → Developer mode → Load unpacked.
2. Click GG on a page, or press **Alt+Shift+G**.
3. On first use, open **Settings & recent captures**, then **Connect to GG**. Enter the GG address and a code from `gg pair` on the controller. Pairing survives server restarts. Codes expire after ten minutes and are single-use.
4. Optionally enter instructions, then press **Enter** or click **Capture**. **Shift+Enter** inserts a new line. No type, destination or image selection is required.

For remote connections, see the [deployment guide](deployment.md#remote-connections).

For example: “Save each painting as a separate record, including its attribution.” Without instructions, one page normally produces one coherent record with several relevant images. Unsure classifications go to searchable **Inbox**. Existing source instructions carry forward on recapture unless replaced.

For a text instruction set, try: “Save the five tips as actionable instructions, without images, under Ideas/SoloDev.” To create that folder, also enter `Ideas/SoloDev` under **Create new folders** (or use `--create-destination Ideas/SoloDev` in the CLI). List missing parents first. Capture can create only these explicitly approved paths, at most 16 per job; it cannot rename, move or delete existing records. Recapture keeps an existing record in its current folder; move it with `gg do` if needed. When the agent chooses a text-only idea record, irrelevant image failures do not make it partial. Missing relevant images and truncated source content still produce a partial result. The browser may still collect candidate images before the agent decides what is relevant.

Appearance defaults to **Dark**. Choose **Dark**, **Light** or **System** on the settings page; the choice applies to both settings and the popup, and System follows your OS/browser preference.

The extension sends visible text, sanitized HTML, captions, an already-visible transcript, and up to 24 image candidates with bytes when obtainable. It excludes form/editor contents, hidden content, scripts, browser storage and cookie stores. Page content itself can contain private information and is sent to your configured model provider. Extraction is best effort. Instructions can ask for captured discussion context; GG does not crawl unloaded replies. For YouTube, open **Show transcript** first; GG does not transcribe audio.

Cross-origin restrictions or network failures can prevent image downloads. GG records missing images when they are relevant to the saved record; available content is still saved. To supply previously missing bytes, make a new capture from the original page. **Retry unfinished work** reuses the already-received snapshot and cannot fetch missing browser images. After acceptance you can close the popup. The settings page shows recent captures, retry and cancellation. Capture devices see and control only jobs submitted by that device. Management devices can inspect all jobs, including jobs created before device ownership was recorded. Re-pairing creates a new device identity; use management pairing to inspect older jobs. Usage notifications are optional while that page is open.

Browser source URL aliases (YouTube video links and X/Twitter post links) reuse the submitted snapshot. Model-facing page/HTML reads are bounded and paginated; complete supplied source text remains preserved. Transcript captures do not automatically attach thumbnail previews. The configured model input limit still applies to unusually large or repeated reads.

## Shared capture instructions

Editing capture instructions in the extension requires management pairing (`gg pair --scope manage`).

`CAPTURE.md` in the vault root, with optional overrides in destination folders, controls the default level of detail and writing style for new captures and recaptures. GG creates it with useful defaults if it is missing, without replacing an existing file. For lists, those defaults ask for every substantive tip, an explanation of each, and concrete actions or examples. Tutorials retain steps, prerequisites and caveats; essays retain their argument and supporting points.

Edit the file directly, or open the extension's **Settings & recent captures → Capture instructions**, edit, and click **Save instructions**. Both edit the same server-side file, shared by all your devices. No service restart is needed. The editor detects stale copies rather than overwriting someone else's changes; reload explicitly after a conflict. Settings edits appear in GG history and can be undone. The limit is 16,000 UTF-8 bytes; an empty file disables the shared defaults.

Instructions for an individual capture override these defaults. Saved source instructions still carry forward on recapture when no new instructions are supplied. Changes do not rewrite existing records automatically, and recapture continues to preserve human edits. Source pages cannot edit `CAPTURE.md`; the worker receives read-only guidance. Folder guidance is pinned when first loaded and reused for retries of that capture. Paired browser devices may edit these capture preferences, without gaining general management permissions. The file expresses content/style preferences, not additional tool permissions or authorization to rearrange the vault.

Folder guidance inherits from root → parent → child, for example `CAPTURE.md` → `subvaults/Art/CAPTURE.md` → `subvaults/Art/Paintings/CAPTURE.md`. In extension settings, choose **Apply to** to edit one file; **Effective instructions** shows the combined guidance. A missing or empty child file adds no overrides. Individual capture instructions take priority. Files are limited to 16 KB each and combined guidance to 64 KB. Folder moves carry their files and change inherited defaults for future jobs. No installation or vault migration is needed.

## Nested folders and automatic routing

GG discovers existing folders under your vault's `subvaults/` directory on each operation. For example:

```sh
mkdir -p "$HOME/Gewolbe/subvaults/Photography/Historic"
```

The next capture can automatically choose `Photography/Historic`. No registration or restart is needed. You can also use `gg do 'Create Historic inside Photography'`; the agent uses the full relative path. Parent folders may still receive captures themselves. GG creates `items/` inside a manually created destination when it first saves a record there.

Hidden folders, symlinks and the reserved `items/` trees are excluded from discovery. Paths support up to 16 levels, 100 characters per folder name and 500 characters overall. Existing vaults keep their layout; no reinitialization is needed.

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

## Portable vault

Open the vault directory in Obsidian and start with `GG Index.md`. Records contain native YAML properties, editable summaries, relative image embeds and preserved source links. `.gg-assets.json` tracks originals, `.gg-history` holds restoration data, `.gg-jobs` holds receiver state and `.gg-proposals` holds confirmation previews. `.gg-plans` preserves multi-record plans and per-snapshot completion across retries. Copy the whole vault to retain content and history. Keep credential state separate.

Your controller is the authoritative writer; laptop/desktop copies can be refreshed using existing file-transfer tools. Arbitrary offline edits are not automatically merged. See [deployment and mirror procedure](deployment.md). Built-in encrypted backups are not implemented.

Browser snapshots and file imports are processed without public network access. Missing images must be supplied by recapturing in the browser. Public URL captures collect their source and media before reading private folder instructions; afterward, network access closes for the rest of that job. Capture cannot search or read unrelated archive records.
