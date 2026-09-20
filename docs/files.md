# Import and export

[Home](../README.md) · [Capture guide](capture.md) · [Usage budgets](cli.md#usage-controls)

## Supported inputs

| Input                                                        | How to add it                            | Current support                                                         |
| ------------------------------------------------------------ | ---------------------------------------- | ----------------------------------------------------------------------- |
| Web pages, including signed-in pages                         | Browser extension                        | Captured text, structure and relevant images; extraction is best effort |
| Public URLs                                                  | `gg capture URL`                         | Server-side fetching; no browser login session                          |
| Local JPEG, PNG or WebP images                               | `gg import PATH` or settings-page upload | Original bytes preserved; 64 MB per file                                |
| UTF-8 Markdown, TXT, RST, CSV, TSV, JSON, YAML, TOML and LOG | `gg import PATH` or settings-page upload | Exact original preserved; 64 KB per file                                |
| Saved GG browser-snapshot JSON                               | `gg capture-file FILE`                   | Local administrative command, not general document import               |
| PDF, office documents, audio, video, RAW or HEIC files       | —                                        | No direct file import yet                                               |

Photos, screenshots and scans can be imported; images need not depict artworks. The current interpretation prompt nevertheless contains artwork-specific assumptions—see [Attribution and current specialization](#attribution-and-current-specialization).

## Importing local files

On the machine containing your files, install the CLI and use `gg connect SERVER_URL` to pair it with your server. Then run:

```sh
gg import ~/Pictures/Collection --instructions "Organize these under References"
gg import notes.md checklist.txt data.csv --instructions "Organize these under Notes"
gg import photo.jpg scan.png --instructions "Create Ideas/References and save these there"
```

Directories are scanned recursively. JPEG, PNG and WebP files up to 64 MB each and UTF-8 text files up to 64 KB each are supported; hidden directory entries, symbolic links and unrelated files inside directories are skipped. Explicitly supplied unsupported files produce an error. Originals stay on your machine and are copied unchanged into records; the server receives the filename, file contents and instructions, not your full local path. Embedded image metadata remains part of the original file.

Each file defaults to one record. The agent inspects it and chooses an existing destination, using `CAPTURE.md` plus your instructions. This consumes normal model usage. Files upload and process one at a time through the server's capture queue and shared limits. The command prints each filename and waits for its result before sending the next file. By default, a failure or usage pause stops the batch. For local imports, `--continue-on-error` continues past unsuccessful model results; usage pauses and input/transport errors still stop it.

To permit new folders, pass `--create-destination PATH` for each exact path (missing parents first), or fill in the new-folder paths field in the extension import panel. Free-text instructions alone do not authorize folder creation.

Run the same command again to resume: completed files are skipped by exact file contents, even after renaming. Unfinished uploads/jobs reconnect or retry. Changing instructions does not rewrite already imported records; use `gg do` to change those records. Ctrl+C stops the CLI; an already accepted server job can finish. If the CLI stops waiting after 15 minutes, rerun it to reconnect. Without a paired server, import uses the configured local vault and the same worker/usage safeguards (`--local` explicitly selects this mode).

Text formats: `.md`, `.markdown`, `.txt`, `.rst`, `.csv`, `.tsv`, `.json`, `.yaml`, `.yml`, `.toml` and `.log`. Files must be nonempty, valid UTF-8 (an optional UTF-8 BOM is preserved), without binary control characters. Invalid encodings, oversized files and unsupported explicit inputs fail visibly rather than being silently truncated. Text files use document instructions, not artwork research. Their contents, including front matter or embedded instructions, are untrusted source material; they cannot configure GG. Links and Markdown images are not automatically downloaded.

GG keeps exact original bytes as `original.<extension>` alongside `source.md` and a generated `record.md`. The import filename supplies initial context; stored original filenames are normalized. JSON/YAML/CSV are read as text, not executed or imported as GG configuration. Summarization remains bounded by the normal model/context limits; this is not unlimited long-document processing.

This does not import PDF/office documents, RAW/HEIC files, or reconcile files manually dropped into the vault. `capture-file` remains the browser-snapshot JSON command.

## Browser import and archive downloads

The extension settings page accepts text and image uploads through a file picker or drag-and-drop. Files upload and process one at a time under normal GG limits. Keep the page open to send remaining files; **Stop after this file** stops the batch without cancelling the accepted job. Select the same files again to resume: stored originals are skipped. A usage pause stops the batch rather than spending beyond your limits.

**Archive** searches saved records without a model call. **Download bundle** produces a `.tar.gz` archive containing `record.md`, preserved source, original files and a preview when present. Relative media links work after extraction. **Originals only** omits the generated record, source copy and previews; imported Markdown originals are included. Downloads exclude credentials, internal state and history, and are currently limited to 128 MB per record bundle. The CLI streams downloads; the browser buffers one bundle before saving.

Archive browsing/downloads require management pairing. A capture-only extension can still capture and import; to enable archive access, run `gg pair --scope manage` on the server and reconnect the extension using that code. Pairing remains durable afterward. Use `gg list` or archive search to obtain a record ID for CLI downloads:

```sh
gg download RECORD_ID --to record.tar.gz
```

## Attribution and current specialization

Local-image imports distinguish each title, artist and year as filename-derived, uncertain, or source-supported. A source-supported value requires a source URL and an excerpt actually fetched during the job, containing the claimed value. Artist/year fields remain empty when only filename hints are available; hints remain in the attribution properties and readable record. Source-supported means cited evidence, not guaranteed authentication: matching that evidence to the pictured work remains the agent's responsibility.

File imports have no public network access. The agent preserves the image and marks attribution uncertain when supplied evidence is insufficient; it cannot verify a filename against a museum website. Capture a relevant source page separately if you want to preserve supporting evidence. Artwork creation dates are distinct from download or exhibition dates. Import routing prefers a suitable existing child folder (for example Art/Paintings) over its parent. Existing imports are not automatically reprocessed.

**Implementation limitation:** all local-image imports currently receive a hardcoded artwork prompt, including attribution preferences and Art/Paintings routing examples. Filename/title/creator/year evidence validation also applies to every local-image import. These are not activated only when a folder is named Art. Folder `CAPTURE.md` files add guidance, but do not currently remove that built-in bias.

A future change should keep general provenance and uncertainty checks in code while moving artwork-specific research and writing preferences into destination policies. Text import has a separate document path. PDF and office-document support would still require additional readers and preservation rules.
