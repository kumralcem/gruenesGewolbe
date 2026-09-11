# Grünes Gewölbe

A file-based personal archive. Pi handles capture and natural-language retrieval; a trusted controller owns the vault. See [README](README.md) for current implementation and [decisions](docs/adr/) for agreed behavior.

## Domain language

- **Vault**: the portable directory containing saved items, records, and existing destinations. No database is required.
- **Subvault**: a user-created destination such as Paintings, Photography, Sculptures, or Ideas. Agents may choose one but cannot create/delete destinations.
- **Saved Item**: one thing worth keeping, stored in an **Item Folder** with its `record.md` (**Item Record**) and preserved content.
- **Visual Capture**: one **Selected Image**, preserving that exact photograph/viewpoint/composition. It excludes discussion, replies, and alternate depictions.
- **Primary File**: the chosen main image. **Preserved Files** include the initially obtained copy and any conservatively confirmed better version. Earlier copies are retained.
- **Thumbnail Preview**: a bounded derivative for display. It never replaces a preserved original.
- **Idea Source**: source text or an existing video transcript, its URL, and a useful **Summary**. The summary supports search by remembered problems; instructional material retains usable steps and conditions.
- **Source Copy**: readable preserved source text, independent of the generated summary or focus.
- **Capture Date**: when the item was saved. **Source Publication Date**: when the source was published, if known. Neither implies continuing accuracy.
- **Browser Snapshot**: content collected by an explicit extension action from the current browser session. It contains page text, sanitized HTML, an available transcript, and optionally one image's bytes; no cookie store or browser credentials.
- **Capture Job**: a persisted receiver input and its processing status. Pending jobs run sequentially; failed/interrupted jobs can be retried.
- **Capture Queue**: unresolved user decisions, such as an unsuitable destination or ambiguous image. Distinct from background processing jobs.
- **Vault Problem**: a malformed or unsafe entry reported locally without hiding valid items elsewhere.
- **Tag**: an open-ended search label, written as an Obsidian-compatible tag in the Item Record.
- **GG Index**: a generated Markdown list of relative links to item records, rebuilt from the files. It is not the source of truth.

## Boundaries

The browser uses its own session to obtain content. The worker receives that content, never the authenticated profile. The controller holds model keys and the writable vault. `ask` has only search/read access. Only explicit user setup creates destinations. Existing live-archive import and hosted deployment are separate future work.
