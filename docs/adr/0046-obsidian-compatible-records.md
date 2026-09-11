---
status: accepted
---

# Obsidian-Compatible Item Records

The user wants to open the file archive directly in Obsidian. New items retain one `record.md` beside preserved content in a shallow per-item directory. No database, Obsidian plugin, or export process is required.

Records use native YAML [properties](https://help.obsidian.md/properties): identity, kind, title, title alias, existing subvault, source URL, capture date, known publication date/creator/year, tags, and relative file pointers. Long summaries live in the Markdown body. Internal asset hashes/dimensions live in `.gg-assets.json` so the properties UI does not need to edit nested object structures.

Readable records contain a title, summary, source link, and relative Markdown [links](https://help.obsidian.md/links) and [image embeds](https://help.obsidian.md/embeds). Ideas preserve the source in `source.md`. `GG Index.md` provides meaningful titled links even though per-item notes are named `record.md`. All content links remain valid when the whole vault is copied.

GG reads YAML after external edits. Automatic image upgrades update managed file pointers and a marked media block while retaining user properties, edited summaries, and notes. The generated index may be rebuilt; user-written index notes should be separate. Required identities and GG-managed paths still matter to GG, so arbitrary renames are not automatically reconciled.

Earlier Pi JSON/YAML records remain readable. This change applies to new captures and does not perform bulk conversion of the user's existing archive or the removed Rust application's records.
