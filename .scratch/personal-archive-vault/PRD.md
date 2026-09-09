Status: ready-for-agent

# Personal Archive Vault PRD

## Problem Statement

The user currently saves interesting links, artwork, photos, posts, and research sources by sending links to themselves or manually curating files later. This workflow breaks down because curation is cumbersome, source content disappears, duplicate saving is hard to detect, and future rediscovery is unreliable.

The user wants a personal archive that behaves like a durable local vault: normal files on disk, copyable and inspectable without the app, with a desktop interface for saving, browsing, searching, tagging, reviewing, and enriching saved material. The first version should focus on artwork/image archiving and basic idea/source captures while preserving a path toward vector search, image-search enrichment, agent access, and richer automation.

## Solution

Build a Tauri desktop app backed by a reusable Rust archive core and a minimal CLI. The app manages one active vault at a time, while remembering multiple vault roots. A vault contains subvaults such as Paintings, Historical Photos, Memes, Webcomics, and Idea Sources. Each saved item has one home subvault, one ordinary item folder, a Markdown item record with structured frontmatter, preserved files, and optional source copies.

The vault files are canonical. Derived indexes, embeddings, cached thumbnails, and app caches are rebuildable and may live in hidden app state inside the vault. The first milestone supports opening and creating vaults, basic Paintings import, URL-based capture, manual fallback for hostile sources, offline browsing/editing/search, OpenAI-backed provider configuration, and metadata text search. Vector search, image-search identification, deep enrichment, and broader provider support remain later work.

## User Stories

1. As a user, I want to create a vault as an ordinary local folder, so that I own the archive independently of the app.
2. As a user, I want to open an existing vault, so that copied or restored vaults remain usable.
3. As a user, I want to switch active vaults from the desktop app, so that I can keep separate portable archives.
4. As a user, I want the app to rebuild missing or stale indexes from vault files, so that hidden app state is never required to make the vault usable.
5. As a user, I want a vault to contain subvaults, so that different archive areas can have different defaults and views.
6. As a user, I want each saved item to have one home subvault, so that physical ownership stays simple.
7. As a user, I want items to link to related items, collections, notes, or URLs, so that cross-subvault relevance is preserved without duplicate files.
8. As a user, I want to import my existing paintings folder, so that my current archive moves into the vault model.
9. As a user, I want imports to copy files by default, so that the original folder remains intact until I verify the result.
10. As a user, I want imported files to preserve original filename and source folder provenance, so that import history remains auditable.
11. As a user, I want imported paintings to become item folders and item records, so that they are immediately browseable and searchable.
12. As a user, I want imported paintings to be marked for review when metadata is uncertain, so that I can clean them up later.
13. As a user, I want to paste a URL plus an optional saving reason, so that capture stays low-friction.
14. As a user, I want a capture to succeed even when metadata extraction is incomplete, so that saving is not blocked by uncertainty.
15. As a user, I want visual captures to preserve the best file available from the source, so that the saved item is media-first.
16. As a user, I want preserved media files to keep their original formats, so that the archive records what I actually saved.
17. As a user, I want enrichment to add better-file candidates without silently replacing the primary file, so that automation cannot overwrite my intent.
18. As a user, I want folder names for Paintings to prefer creator, year, and title, so that files remain recoverable in the filesystem.
19. As a user, I want stable item IDs inside item records, so that collections, indexes, and links do not depend on folder names.
20. As a user, I want folder rename suggestions when metadata improves, so that paths are not silently moved.
21. As a user, I want basic idea captures from URLs, so that sources for essays, product-release notes, and useful posts can live in the same vault system.
22. As a user, I want idea captures to preserve cleaned text when possible, so that the useful source content survives if the page disappears.
23. As a user, I want cleaned text to include full main content, so that extraction does not discard context I may need later.
24. As a user, I want hostile-site capture to have a clipboard-assisted fallback, so that I can still save X posts and other blocked sources without managing downloaded files.
25. As a user, I want manual fallback to accept URL, saving reason, copied image, and copied text, so that the saved item still has provenance and context.
26. As a user, I want surrounding discussion excluded by default, so that comment noise does not pollute the archive.
27. As a user, I want comments or replies saved separately when I care about them, so that valuable discussion can still be archived intentionally.
28. As a user, I want summaries to explain why saved items matter, so that future rediscovery is actionable.
29. As a user, I want my saving reason to be preserved and prioritized, so that my intent outranks inferred metadata.
30. As a user, I want AI-suggested tags, so that capture produces useful classification without manual work.
31. As a user, I want tags normalized against an existing tag registry during capture, so that obvious duplicates are reused.
32. As a user, I want genuinely new tags to be allowed, so that the vault can grow organically.
33. As a user, I want collections separate from tags, so that intentional project bundles have names, purposes, and curated item lists.
34. As a user, I want collection membership visible from items, so that items can be discovered through collections and search.
35. As a user, I want a lightweight review queue, so that only uncertain or important cases need attention.
36. As a user, I want high-confidence metadata accepted automatically, so that the vault stays low-maintenance.
37. As a user, I want low-confidence or high-impact metadata staged as suggestions, so that important facts are not silently wrong.
38. As a user, I want duplicate candidates detected and warned about, so that I stop saving the same artwork repeatedly.
39. As a user, I want duplicate detection to be local-first and cheap, so that every capture does not require paid AI.
40. As a user, I want exact metadata search in the first milestone, so that I can find items by title, creator, year, tags, reason, summary, URL, filename, and collection.
41. As a user, I want cached thumbnails, so that gallery browsing is fast.
42. As a user, I want a dense workbench interface, so that browsing, review, and editing are efficient.
43. As a user, I want image grids for visual subvaults, so that Paintings and similar areas are easy to scan.
44. As a user, I want a searchable list/detail view for idea sources, so that text-heavy saved items are easy to inspect.
45. As a user, I want to edit item records through the UI, so that normal cleanup does not require opening Markdown manually.
46. As a user, I want direct file edits to remain valid, so that the vault stays file-first.
47. As a user, I want the app to refresh indexes after file edits, so that UI/search follows the canonical records.
48. As a user, I want provider configuration in user-specific app state, so that vault copies do not leak API keys.
49. As a user, I want OpenAI support first, so that AI summaries, tags, and vision metadata work without building a provider router immediately.
50. As a user, I want AI budget modes and cost logs, so that AI capture is useful without hidden spend.
51. As a user, I want the app to work without AI, so that browsing, saving, editing, import, and metadata search remain available offline.
52. As a user, I want AI calls to send only minimal needed files or text, so that privacy and cost are controlled.
53. As a user, I want compact metadata provenance, so that important inferred fields can be audited later.
54. As a user, I want an activity log for captures, imports, enrichment, errors, index rebuilds, and AI costs, so that debugging and auditing are possible.
55. As a user, I want a minimal CLI over the archive core, so that agents and scripts can validate, import, rebuild indexes, search, and inspect records.
56. As an agent, I want to use the CLI and item records instead of driving the GUI, so that archive operations are scriptable and token-efficient.

## Implementation Decisions

- The vault is the source of truth. Databases, indexes, embeddings, thumbnails, and caches are derived and rebuildable.
- The app must support one active vault at a time while allowing multiple vault roots to be opened over time.
- The vault contains multiple subvaults. Each saved item has exactly one home subvault.
- Cross-subvault relevance is represented through collections, item links, tags, and search rather than multi-home items or duplicated files.
- Saved items are represented as ordinary item folders plus Markdown item records with structured frontmatter.
- Item folders use readable names. Stable IDs live inside item records.
- Paintings prefer the readable name pattern of creator, year, and title, with explicit unknown fallbacks.
- Item records are canonical and should be editable both through the UI and directly as files.
- Collections are durable files with item backreferences, not only tag-like fields.
- Tags are open-ended but normalized during creation through a vault-level tag registry.
- Preserved files keep their original formats. Generated thumbnails, previews, converted files, and better-file candidates are derived or additive.
- Enrichment can add candidate files, metadata suggestions, tags, summaries, or better source links, but changing the primary file requires user approval.
- Basic Paintings import is in the first milestone. Image-search or reverse-lookup identification is noted as later enrichment.
- Import copies files by default and records import provenance.
- Basic idea capture is in the first milestone. Complex crawling, browser automation, and agentic research are out of the first milestone.
- Idea captures preserve cleaned text as the primary source copy when possible, keeping full main content.
- Best-effort extraction should support X.com and similar sources only within normal access limits.
- Manual fallback accepts a structured bundle: source link, saving reason, copied image, and copied text.
- Surrounding discussion is excluded by default and should be saved as a separate saved item when valuable.
- Duplicate detection is local-first and cheap during capture. Paid AI is reserved for enrichment or ambiguous duplicate candidates.
- The first milestone implements metadata text search before vector search.
- Vector search should later embed compact item records first, not full source copies.
- AI provider configuration is user-specific app state. API keys and provider settings do not live in the vault.
- OpenAI is the first AI provider, behind a provider boundary that can support other providers later.
- AI budget modes govern capture and enrichment depth, including vision calls for image captures/imports.
- AI calls use minimal task context and should not upload whole vaults or unrelated item records.
- Metadata provenance should be compact and inspectable without cluttering the default UI.
- The activity log is secondary audit/debug material, not an event store required to reconstruct the vault.
- The desktop app is a Tauri shell. Archive rules live in a reusable Rust archive core.
- The project starts as a Rust workspace with an archive core, a minimal CLI, and a Tauri desktop app.
- The minimal CLI should cover vault validation/opening, Paintings import, index rebuild, metadata search, and item inspection.
- The first UI should be a workbench: subvault navigation, collections, review filters, visual grids, searchable lists, and item details.

## Testing Decisions

- The highest-value test seam is the archive core public API. Tests should exercise externally visible vault behavior rather than implementation details.
- Archive core tests should create temporary vaults, perform operations, and assert on resulting item folders, item records, tag registry updates, collection files, derived index behavior, and activity log entries.
- Import tests should verify copy-by-default behavior, import provenance, readable item folder creation, review status, and preservation of original file formats.
- Capture tests should verify URL-plus-saving-reason handling, offline/manual capture, cleaned text preservation, manual fallback records, and source-link provenance.
- Duplicate detection tests should verify local signals and warnings without requiring paid AI calls.
- Index tests should verify that missing or stale derived indexes can be rebuilt from canonical vault files.
- Search tests should verify metadata text search over titles, creators, years, tags, saving reasons, summaries, source links, original filenames, and collections.
- Tag tests should verify that new tags are compared against the tag registry and existing aliases before being added.
- Review tests should verify that uncertain imports, failed extraction, manual fallback, duplicate candidates, and low-confidence metadata suggestions enter the review queue.
- AI integration should be tested behind a provider interface with fakes for most tests. Provider configuration and cost logging should be covered without relying on live OpenAI calls in normal automated tests.
- Tauri command tests should be thin and confirm that UI-facing commands call archive core behavior correctly.
- CLI tests should use the same archive core seam and verify command-level behavior for validation, import, rebuild, search, and inspect.
- UI tests should focus on workbench workflows at a high level: open vault, browse subvault, search, inspect item details, edit metadata, review item, and import paintings.
- For the first usable Linux workbench, run browser-level Playwright tests against controlled Tauri-command responses and keep a manual smoke checklist for the real Tauri window. Revisit automated native-window end-to-end testing once the Offline Archive Loop is stable and its maintenance cost can be justified.

## Out of Scope

- Browser extension capture.
- Pure native Rust UI.
- Electron implementation.
- Treating a hidden database as canonical archive storage.
- A continuously running background service required to keep vaults functional.
- Full web-page replay or perfect raw HTML archiving.
- Logged-in scraping, bypassing platform restrictions, or deep X.com harvesting.
- Archiving comments, replies, and surrounding discussion by default.
- Automatically replacing preserved files with enriched alternatives.
- Required paid AI calls for basic save, browse, import, edit, or metadata search.
- Full vector search in the first milestone.
- Full-source-copy embeddings in the first milestone.
- Image-search or reverse-lookup identification in the first milestone.
- Multi-provider AI routing in the first milestone.
- Deep agentic enrichment, broad web research, or automated source hunting in the first milestone.
- A full CLI parallel to the desktop app.
- A heavy graph model for typed item links.

## Further Notes

Milestone one should prove the core loop: create/open a vault, import existing paintings, capture basic URLs and manual fallback bundles, browse visual and idea subvaults, edit item records, review uncertain items, search metadata, configure OpenAI, and run bounded AI summaries/tags when enabled.

Later milestones should consider vector search over compact item records, reverse image search for imported paintings and enrichment, richer duplicate detection with local visual embeddings, batch enrichment, collection curation tools, better agent-facing CLI commands, additional AI providers, and selective full-source-copy embeddings if compact records miss important retrieval cases.

The main product constraint is low maintenance. The app should do useful work automatically when confidence is high, but preserve user intent and source provenance when automation is uncertain.
