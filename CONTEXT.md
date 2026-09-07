# Personal Archive

A personal archive for saving, describing, and rediscovering material found while browsing.

## Language

**Vault**:
The user's durable personal archive: a local collection of saved items and their descriptive records that can be copied or backed up as ordinary files.
_Avoid_: Database, library, export

**Active Vault**:
The vault currently opened by the app. The app may remember multiple vault roots, but capture, browsing, and indexing operate on one active vault at a time.
_Avoid_: Workspace, account, profile

**Subvault**:
A named home area within a vault for a coherent archive use, such as paintings, historical photos, memes, webcomics, or video essay sources. Subvaults may have their own defaults for item types, records, naming, and views while still sharing the vault's search, tags, collections, links, and indexes.
_Avoid_: Separate vault, folder, category

**Home Subvault**:
The single subvault where a saved item's item folder physically lives. Cross-subvault relevance should be expressed through collections, item-record links, tags, or search rather than multiple home subvaults.
_Avoid_: Membership, view, location

**Vault Trash**:
The visible, durable area inside a vault that holds recoverably removed item folders outside normal browsing and search. Its layout preserves each removed item's home subvault so the item can be restored.
_Avoid_: Deleted items, recycle bin, derived state, trash file

**Vault Problem**:
A localized condition where one canonical vault file cannot be interpreted or used. A vault problem is reported with its file path without preventing valid saved items elsewhere in the vault from remaining available.
_Avoid_: Review reason, invalid vault, app error

**Item Folder**:
The ordinary filesystem directory that stores one saved item's files and readable record. Item folders are a recovery format for ownership and backup, while the app interface is the primary way to browse and search the vault.
_Avoid_: Album, directory, container

**Item Record**:
The human-readable metadata document inside an item folder. The item record describes the saved item with structured fields and readable sections such as saving reason, summary, notes, and source details.
_Avoid_: Manifest, metadata file, database row

**Item Record Conflict**:
A condition where an item record changed after the structured editor loaded it, making an ordinary save unsafe. The user must reload the external version or deliberately overwrite it before editing can continue.
_Avoid_: Sync conflict, vault problem, validation error

**Derived Index**:
A rebuildable search aid generated from the vault, such as a text index, compact catalog, or embedding store. A derived index may be deleted and recreated without losing archive meaning.
_Avoid_: Source of truth, catalog, archive database

**Thumbnail Preview**:
A bounded-size, rebuildable visual representation used for fast gallery browsing. It is derived from a preserved file and is never the saved item's primary or preserved file.
_Avoid_: Preserved file, primary file, copied original

**Activity Log**:
A secondary append-only record of captures, imports, enrichment runs, errors, index rebuilds, and AI costs. The activity log supports audit and debugging but is not required to reconstruct the vault.
_Avoid_: Event store, history, source of truth

**Offline Archive Loop**:
The complete local workflow of opening or creating a vault, adding saved items, browsing their preserved files, editing their item records, resolving review status, and reopening the vault later without network or AI access.
_Avoid_: Demo flow, backend milestone, scaffold

**Saved Item**:
A thing the user wants to keep and find again, such as an artwork, post, article, source, note, or media file. A saved item may come from a URL, but the URL is not itself the saved item unless the user only wants a bookmark.
_Avoid_: URL, page, link

**Artwork Saved Item**:
A saved item whose durable value is primarily a visual artwork or image. Artwork saved items prioritize the best available image file and descriptive metadata such as artist, title, year, style, subjects, and emotional qualities.
_Avoid_: Painting, photo, image

**Idea Source**:
A saved item whose durable value is the content or argument of a post, blog article, or web page. It keeps locally readable source material and a separate summary; a source link alone does not preserve the idea.
_Avoid_: Bookmark, URL, image capture

**Best Available File**:
The highest-quality local file the app can reasonably obtain for a saved item. For visual saved items this may be the largest image from the source link at first, with broader web search treated as an optional enrichment step.
_Avoid_: Original, asset, download

**Preserved File**:
The file saved from the user's chosen source at creation time. Preserved files keep their original format and should not be silently replaced by later enrichment.
_Avoid_: Converted file, preview, derivative

**Enrichment**:
A follow-up improvement to a saved item after the initial save, such as finding a higher-quality file, improving metadata, adding tags, or refining a summary.
_Avoid_: Ingestion, cleanup, research

**AI Budget Mode**:
The user's selected cost and depth setting for AI-assisted capture or enrichment. AI budget modes make the tradeoff between cheap capture, richer metadata, and deeper research visible.
_Avoid_: Quality setting, model setting, automation level

**Primary File**:
The file the saved item treats as its main local representation. Enrichment can add candidate files, but changing the primary file requires user approval.
_Avoid_: Best file, latest file, replacement

**Source Link**:
The original URL or reference where a saved item was found. Source links provide provenance and a way back to the original, but they are not the durable archive record.
_Avoid_: Bookmark, capture

**Import Provenance**:
The local origin details for an imported item, such as original filename, source folder, and import date. Import provenance helps identify items and audit what happened during import.
_Avoid_: Source link, file history, import log

**Imported Item**:
A saved item created from an existing local file or folder rather than a source link. Imported items may lack source links and often need identification, tagging, and review after import.
_Avoid_: Migrated file, existing file, upload

**Import Run**:
A user-initiated attempt to add supported files from one source folder. An import run may produce saved items, skips, duplicate candidates, and individual failures without treating the whole run as one transaction.
_Avoid_: Batch transaction, migration

**Source Copy**:
A local preservation copy of source material kept so the saved item remains inspectable if the original disappears. Source copies support the saved item, but they are usually less important for day-to-day use than the item's summary and descriptive metadata.
_Avoid_: Mirror, scrape, backup

**Cleaned Text**:
Readable source text with page chrome, navigation, scripts, ads, and unrelated clutter removed. Cleaned text is the preferred source copy for idea captures.
_Avoid_: Raw HTML, screenshot, page dump

**Summary**:
A compact explanation of a saved item's meaning, why it matters, and its likely future use. For an Idea Source, it explains the source's central point without replacing the preserved source text; summaries are the primary surface for rediscovery and later action.
_Avoid_: Abstract, excerpt, description

**Review Status**:
A summary of whether a saved item has unresolved review reasons, such as incomplete metadata, manual fallback, duplicate candidates, or metadata suggestions. An item becomes reviewed only when each reason has been accepted, corrected, or dismissed.
_Avoid_: Workflow status, task state, error

**Review Reason**:
One specific concern requiring a user decision about a saved item, such as unknown metadata, a duplicate candidate, a metadata suggestion, or an unavailable thumbnail preview. Each review reason is resolved independently by accepting, correcting, or dismissing it.
_Avoid_: Review status, error, task

**Metadata Suggestion**:
An inferred or AI-generated field value that needs user attention because it is low-confidence, conflicting, or would change an important fact. High-confidence metadata may be accepted automatically so review remains an exception path.
_Avoid_: Draft metadata, AI output, proposed edit

**Metadata Provenance**:
Compact evidence about where an item record field came from, such as user input, source text, import provenance, local extraction, or AI inference. Metadata provenance helps audit important fields without making the normal UI noisy.
_Avoid_: Audit log, citation, explanation

**Duplicate Candidate**:
A saved item or incoming capture that appears to overlap with an existing saved item based on source links, file fingerprints, or descriptive metadata. Duplicate candidates should warn the user without blocking capture automatically.
_Avoid_: Duplicate, collision, conflict

**Exact File Duplicate**:
An incoming imported file whose content fingerprint is identical to a preserved file already in the vault. Exact file duplicates are skipped by default rather than creating review work, while the user may explicitly import them anyway.
_Avoid_: Duplicate candidate, metadata match, filename match

**Saving Reason**:
The user's short explanation of why a saved item is worth keeping or how they expect to use it later. When present, the saving reason is more authoritative than inferred summaries or tags.
_Avoid_: Note, caption, prompt

**Manual Fallback**:
A capture path where the user supplies source material directly, such as copied image data or copied post text, when the app cannot reliably extract it from the source link. Manual fallback still keeps the source link and saving reason as provenance.
_Avoid_: Upload, attachment, manual entry

**Tag**:
A short label used to group and rediscover saved items by qualities such as style, subject, mood, use case, project, or medium. Tags are open-ended, but new suggestions should be compared against existing tags during creation so obvious duplicates are reused instead of invented.
_Avoid_: Category, label, keyword

**Tag Registry**:
The vault-level record of known tags, aliases, and meanings. The tag registry guides item creation so tags stay normalized while still allowing genuinely new tags to be added.
_Avoid_: Taxonomy, ontology, tag database

**Collection**:
An intentional grouping of saved items for a project, theme, or future use. Collections are separate from tags: tags describe item qualities, while collections gather items into a purposeful bundle without moving their item folders.
_Avoid_: Tag, folder, category

**Item Link**:
A lightly typed relationship recorded in an item record that points to another saved item, collection, note, URL, or related archive area. Item links express relevance without changing an item's home subvault.
_Avoid_: Subvault membership, shortcut, duplicate

**Surrounding Discussion**:
Comments, replies, quotes, or adjacent thread content around a saved item. Surrounding discussion is excluded by default; if it matters, it should be saved as its own saved item.
_Avoid_: Context, thread, comments
