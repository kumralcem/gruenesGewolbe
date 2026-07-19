Status: ready-for-agent

# Offline Archive Loop PRD

## Problem Statement

The repository has a well-tested archive core, CLI paths, desktop-facing adapters, and a static workbench scaffold, but it does not yet provide a functional desktop application. The user cannot launch a real window and complete the basic local archive workflow: create or open a Vault, add paintings, see the preserved images, correct metadata, resolve Review Reasons, close the application, and reopen the same Vault later.

The current prototype also treats several domain behaviors as completed even though they exist only behind Rust APIs or test fakes. Import is shallow and synchronous, thumbnail files are renamed copies rather than bounded previews, Review Status can be cleared without resolving individual concerns, direct file errors can affect broader operations, and the on-disk format is implemented with ad hoc parsing. There are no existing real Vaults, so this is the right point to establish a coherent format and usable Offline Archive Loop before compatibility obligations or external integrations make changes expensive.

## Solution

Build the first functional Linux desktop workbench as a real Tauri application with a modular TypeScript frontend. The application will launch through simple repository scripts, use native filesystem dialogs, remember previously opened Vaults, and automatically reopen the last valid Active Vault.

The workbench will support adding individual image files and recursively importing Paintings folders, browsing actual Thumbnail Previews, searching metadata, editing Item Records through a structured editor, resolving Review Reasons individually, managing recoverable removal through Vault Trash, and reopening the same canonical files after restart. It will remain fully usable without network access or AI.

Replace the disposable prototype schema with format version 2. Item Records remain single Markdown documents with structured frontmatter and readable sections. Real parsers and serializers will replace ad hoc line handling, unknown user-authored content will survive structured edits, and optimistic writes will protect direct filesystem changes. Malformed records will become localized Vault Problems rather than making the entire Vault unavailable.

## User Stories

1. As a user, I want to launch the desktop application through one repository script, so that I can use it without remembering development commands.
2. As a user, I want a separate local build script, so that I can produce and run a release binary without creating an installer.
3. As a Linux user, I want the first application milestone optimized for my current platform, so that packaging work for unused platforms does not delay a functional archive.
4. As a user, I want the application to show only implemented controls, so that unfinished URL or AI features do not create false expectations.
5. As a user, I want to create a Vault through a native folder dialog, so that I do not have to type filesystem paths.
6. As a user, I want Vault creation refused for non-empty folders, so that the application cannot initialize an unrelated directory accidentally.
7. As a user, I want an existing valid Vault to be opened rather than recreated, so that its canonical files remain authoritative.
8. As a user, I want to open an existing Vault through a native folder dialog, so that copied and restored Vaults remain easy to use.
9. As a user, I want previously opened Vaults remembered outside the Vault, so that I can reopen them quickly.
10. As a user, I want the last Active Vault reopened automatically when it remains valid, so that normal startup is low-friction.
11. As a user, I want a missing last Active Vault handled without silently switching or creating anything, so that filesystem changes remain explicit.
12. As a user, I want a repair screen when a recognizable Vault is missing safe structural directories, so that recoverable layout damage does not make the archive inaccessible.
13. As a user, I want Vault repair to show what it will create before acting, so that canonical filesystem mutations remain deliberate.
14. As a user, I want repair limited to recreating safe empty structure, so that malformed records or existing canonical files are never overwritten.
15. As a user, I want to select one image through a native file picker, so that normal day-to-day additions are quick.
16. As a user, I want to select several images through a native file picker, so that I can add a small group without organizing a source folder first.
17. As a user, I want selected files copied into the Paintings Subvault, so that source files remain unchanged.
18. As a user, I want preserved images to retain their original bytes and formats, so that adding them cannot degrade or replace the source material.
19. As a user, I want to import a Paintings folder recursively, so that nested source organization does not hide images from the Import Run.
20. As a user, I want symbolic links skipped during recursive import, so that imports cannot loop or unexpectedly leave the selected source tree.
21. As a user, I want skipped symbolic links and unsupported files reported, so that the Import Run remains auditable.
22. As a user, I want JPEG, PNG, WebP, and GIF files supported initially, so that common downloaded artwork formats can enter the Vault.
23. As a user, I want import to begin without a separate preview step, so that uncertain metadata is handled through review rather than blocking preservation.
24. As a user, I want visible progress during a long Import Run, so that I know the application is working.
25. As a user, I want to cancel an Import Run, so that I can stop an accidental or unexpectedly large operation.
26. As a user, I want cancellation to keep already imported Saved Items, so that completed independent work is not rolled back.
27. As a user, I want one failed source file not to roll back the entire Import Run, so that damaged or inaccessible files do not block valid material.
28. As a user, I want a final Import Run summary showing imported, skipped, exact-duplicate, duplicate-candidate, cancelled, and failed files, so that I can understand the outcome.
29. As a user, I want individual import failures appended to the Activity Log, so that later diagnosis has durable context.
30. As a user, I want an Exact File Duplicate skipped by default, so that re-importing a source folder does not flood the Vault with identical files.
31. As a user, I want an explicit way to import an Exact File Duplicate anyway, so that byte-identical files can remain separate when that distinction matters to me.
32. As a user, I want ambiguous metadata or provenance matches imported as Duplicate Candidates, so that uncertain overlap does not block preservation.
33. As a user, I want imported metadata inferred from filename conventions, basic file facts, and optional user input, so that offline capture remains useful.
34. As a user, I want embedded EXIF, XMP, and IPTC metadata ignored in this milestone, so that unreliable metadata extraction does not add complexity to the Offline Archive Loop.
35. As a user, I want missing or conflicting inferred fields represented as Review Reasons, so that saving succeeds without pretending uncertain metadata is resolved.
36. As a user, I want actual bounded Thumbnail Previews in the Paintings gallery, so that visual browsing is fast and useful.
37. As a user, I want Thumbnail Previews to be derived and rebuildable, so that deleting cached previews cannot damage archive meaning.
38. As a user, I want the detail view to use the Preserved File, so that I can inspect the best local representation rather than only a reduced preview.
39. As a user, I want animated GIF previews to use a stable first frame initially, so that the gallery remains predictable without requiring animation processing.
40. As a user, I want an undecodable image preserved with a placeholder, so that unusual or damaged material is not discarded merely because preview generation failed.
41. As a user, I want preview decoding failures represented as Review Reasons and Activity Log entries, so that they can be found and understood later.
42. As a user, I want the gallery sorted by most recently added by default, so that newly imported material is easy to find.
43. As a user, I want gallery sorting by title, creator, year, oldest, and newest, so that I can browse according to the cleanup task at hand.
44. As a user, I want metadata search across the Active Vault, so that I can rediscover Saved Items without network or AI access.
45. As a user, I want search results to open Item Details, so that search is part of a complete browsing workflow.
46. As a user, I want Vault Trash excluded from normal gallery, collection counts, Review Queue, and search results, so that recoverably removed items do not behave as active archive content.
47. As a user, I want a structured Item Record editor, so that normal cleanup does not require editing Markdown manually.
48. As a user, I want to edit title, creator, year, Saving Reason, summary, and Tags, so that common descriptive fields are manageable in the application.
49. As a user, I want existing Collection membership and Item Links visible in Item Details, so that cross-item context is not hidden.
50. As a user, I want an explicit Save action, so that related field edits become one validated canonical write.
51. As a user, I want invalid field values explained before saving, so that the application cannot produce malformed Item Records silently.
52. As a user, I want user-authored Markdown sections and unknown frontmatter preserved during structured saves, so that direct file editing remains trustworthy.
53. As a user, I want the application to detect an Item Record Conflict, so that a UI save cannot silently overwrite a newer direct file edit.
54. As a user, I want to reload the external version after an Item Record Conflict, so that I can continue from canonical disk state.
55. As a user, I want deliberate overwrite available after reviewing a conflict, so that I can still choose the structured editor's version explicitly.
56. As a user, I want external Item Record edits refreshed when the window regains focus, so that the workbench normally follows filesystem changes without a background service.
57. As a user, I want a manual refresh command, so that I can request an immediate rescan when needed.
58. As a user, I want a suggested readable Item Folder rename after metadata cleanup, so that the filesystem can reflect improved creator, year, and title data.
59. As a user, I want folder renames to show old and proposed paths and require a separate confirmation, so that editing metadata never moves files silently.
60. As a user, I want folder-name collisions resolved with a small unique suffix, so that applying a rename cannot overwrite another Item Folder.
61. As a user, I want each unresolved concern shown as a distinct Review Reason, so that I know why an item needs attention.
62. As a user, I want Review Status derived from unresolved Review Reasons, so that an item cannot be marked reviewed while known concerns remain.
63. As a user, I want to accept, correct, or dismiss each Review Reason independently, so that resolution reflects an actual decision.
64. As a user, I want existing Metadata Suggestions shown without requiring live AI, so that the review model can be completed independently of its future provider.
65. As a user, I want to accept, edit then accept, or dismiss a Metadata Suggestion, so that inferred values remain under my control.
66. As a user, I want accepted Metadata Suggestions to retain Metadata Provenance, so that important inferred facts remain auditable.
67. As a user, I want a Duplicate Candidate resolved as Not a Duplicate, Keep Both, or Move This Item to Vault Trash, so that review does not require a complex merge workflow.
68. As a user, I want recoverable removal instead of immediate deletion, so that an accidental cleanup action does not destroy archive content.
69. As a user, I want recoverably removed Item Folders stored in visible Vault Trash, so that they remain ordinary owned files rather than hidden derived state.
70. As a user, I want Vault Trash organized by Home Subvault, so that restoration retains the item's physical archive context.
71. As a user, I want Collection membership and incoming Item Links retained while an item is in Vault Trash, so that restoration returns it to its previous context.
72. As a user, I want references to trashed items visibly identified, so that retained links do not appear silently broken.
73. As a user, I want to restore an item from Vault Trash, so that recoverable removal is reversible.
74. As a user, I want restoration to use a unique suffix when the original folder name is occupied, so that restore never overwrites another item.
75. As a user, I want Vault Trash never purged automatically, so that retention remains under my control.
76. As a user, I want permanent deletion to require explicit confirmation, so that destructive removal cannot happen casually.
77. As a user, I want permanent deletion to show affected Collections and incoming Item Links, so that I understand its reference impact.
78. As a user, I want permanent deletion to remove known references deliberately, so that the active Vault does not retain known dangling links silently.
79. As a user, I want one malformed Item Record reported as a localized Vault Problem, so that valid Saved Items remain accessible.
80. As a user, I want the path and parse error for a Vault Problem, so that I can correct the canonical file directly.
81. As a user, I want malformed records left untouched, so that the application cannot destroy evidence needed for manual recovery.
82. As a user, I want browsing and index rebuild to skip only malformed records and report omissions, so that one file cannot disable the entire Vault.
83. As a user, I want import summaries, Vault Problems, and contextual errors visible where relevant, so that normal diagnosis does not require reading log files.
84. As a user, I want an Open Activity Log command, so that detailed audit information remains inspectable without building a full log viewer.
85. As an agent, I want structured CLI commands for adding files and listing Vault Problems, so that offline archive operations remain scriptable.
86. As an agent, I want structured CLI commands for moving, restoring, and permanently deleting Vault Trash items, so that recovery workflows do not require GUI automation.
87. As an agent, I want CLI destructive actions to require explicit item identity and confirmation, so that scripts cannot delete vaguely selected content.
88. As a user, I want the application to close and reopen the same Vault without hidden canonical state, so that the complete Offline Archive Loop proves the file-first model.
89. As a user, I want normal browsing, import, editing, review, search, removal, and restoration to work without network access, so that external services never gate ownership of the archive.
90. As a user, I want correct behavior with approximately 5,000 Saved Items, so that the personal archive has comfortable growth headroom without premature infrastructure.

## Implementation Decisions

- The milestone is complete only when a real Linux Tauri window can perform the Offline Archive Loop end to end and reopen the same Vault after restart.
- The desktop application will be a real Tauri executable rather than only a Rust library, static configuration, or frontend-facing adapter.
- The frontend will use Vite and modular TypeScript without adopting a large component framework initially.
- A typed command client will isolate frontend code from direct global Tauri calls and provide one contract that real invocation and tests can implement.
- Tauri command request and response types will be serializable, registered with the runtime, and aligned with frontend TypeScript types.
- The repository will provide a root launch script that validates local prerequisites and starts the application incrementally, plus a build script for a local release binary.
- AppImage, package-manager installation, signing, and automatic updates are not required.
- Linux is the only verified platform for this milestone, while filesystem and command APIs should avoid unnecessary platform coupling.
- Vault selection and file selection will use native dialogs. Manual paths remain supported by the CLI rather than the normal desktop workflow.
- User-specific app state will remember known Vault roots and the last Active Vault; it is not canonical archive state.
- Vault creation will accept a new or empty directory and refuse unrelated non-empty directories.
- Opening a recognizable but structurally incomplete Vault will produce a repair proposal rather than silently mutating or completely rejecting it.
- Repair operations may recreate only safe empty structural directories and must never rewrite malformed Item Records or overwrite canonical files.
- The disposable prototype format version 1 will be replaced by format version 2. No format-version-1 migration is required because no real Vault exists.
- Compatibility obligations begin with the first real dogfood Vault. Later incompatible migrations require an explicit backup and confirmation.
- Each Item Record remains one Markdown file with structured YAML frontmatter and readable Markdown sections.
- Item Records will use proper parsing and serialization instead of ad hoc line manipulation.
- The serializer must preserve unknown frontmatter fields and user-authored Markdown sections when updating app-owned fields or sections.
- Stable item IDs will use a collision-resistant identifier rather than timestamp uniqueness assumptions.
- Structured writes will be atomic and use optimistic version checks to detect Item Record Conflicts.
- External record changes will be detected when the application regains focus and through an explicit refresh command; continuous filesystem watching is deferred.
- Malformed records will produce localized Vault Problems. Browsing and index rebuild will continue with valid records and report skipped paths.
- Import will expose one asynchronous Import Run operation with progress, cancellation, and a structured final summary.
- Folder import will recurse through ordinary nested directories and skip symbolic links.
- File selection and folder import will share the same preservation, duplicate, metadata, review, preview, and activity behavior.
- Import Runs are best-effort rather than transactional. Completed Saved Items remain after cancellation or individual failure.
- Exact content fingerprints will be checked before copying. Exact File Duplicates are skipped by default with an explicit import-anyway override.
- Ambiguous source, provenance, or metadata overlap remains a nonblocking Duplicate Candidate and creates a Review Reason.
- Offline metadata sources are filename parsing, basic file facts, and optional user input. EXIF, XMP, and IPTC extraction are excluded.
- Derived metadata and Review Reasons will retain sufficient provenance to explain why fields were populated or flagged.
- Metadata index rebuild will be batched around Import Runs rather than performed synchronously after every imported file.
- Thumbnail Preview generation will decode supported images and produce bounded-size cached previews under hidden derived state.
- GIF Thumbnail Previews will use the first frame initially.
- Preview decoding failure will not fail preservation. It produces a placeholder, Review Reason, and Activity Log event.
- The workbench will expose dense Paintings browsing, Item Details, metadata search, Review Queue, Vault Problems, and Vault Trash as functional views.
- The default gallery order is newest first, with title, creator, year, oldest, and newest sorting options.
- The structured editor owns title, creator, year, Saving Reason, summary, Tags, and individual Review Reason actions.
- Existing Collection membership and Item Links are visible but collection creation and Item Link editing are deferred.
- Record changes require explicit Save. Folder renaming is a separate confirmed action with old and proposed paths.
- Review Status is derived from unresolved Review Reasons and cannot be cleared independently.
- Metadata Suggestions support accept, edit then accept, and dismiss while preserving accepted Metadata Provenance.
- Duplicate Candidate review supports Not a Duplicate, Keep Both, and Move This Item to Vault Trash. Record merging is deferred.
- Recoverably removed Item Folders live under visible canonical `trash/<home-subvault>/<item-folder>` paths.
- Vault Trash is excluded from normal browsing, collection counts, Review Queue, and search-derived state.
- Collection membership and Item Links remain canonical while an item is in Vault Trash, and references indicate the trashed state.
- Restore never overwrites an occupied path and uses a small unique suffix when needed.
- Vault Trash is never purged automatically.
- Permanent deletion requires reference-impact confirmation and removes known Collection and Item Link references deliberately.
- Import summaries, Vault Problems, and contextual errors are shown in relevant workflows. The full Activity Log remains a file opened through an application command.
- The CLI will gain narrow structured commands for selected-file import, listing Vault Problems, moving items to Vault Trash, restoring items, and permanent deletion.
- The archive core may be reorganized and its prototype APIs may break where needed. Refactoring should follow coherent module boundaries needed by the milestone rather than becoming an unrelated cleanup project.
- Likely focused internal modules include item-record codecs, import processing, preview generation, Review Reasons, Vault Trash, derived search, and activity recording, while the public archive-core API remains the primary behavior boundary.
- The implementation should remain correct around 5,000 generated Saved Items. Optimization beyond batched indexing and bounded preview work should follow measurement rather than assumption.
- URL Capture and AI Enrichment controls will not appear until real integrations exist end to end.

## Testing Decisions

- The primary automated seam is the archive-core public API. Tests should create temporary Vaults, perform user-visible operations, and assert on returned behavior plus canonical filesystem outcomes.
- Archive-core tests should cover format-version-2 creation/opening, Markdown Item Record round trips, unknown-content preservation, atomic writes, Item Record Conflicts, localized Vault Problems, repair proposals, and rebuildable derived state.
- Import tests should cover recursive discovery, symlink skipping, supported and unsupported files, progress events, cancellation, partial failure, exact-duplicate skipping and override, ambiguous Duplicate Candidates, provenance, Activity Log entries, and one batched index refresh.
- Preview tests should use real decodable fixtures and verify bounded derived output, original-byte preservation, GIF first-frame behavior, deletion/rebuild, and graceful decode failure.
- Review tests should verify that Review Status is derived from individual Review Reasons and becomes reviewed only after every reason is accepted, corrected, or dismissed.
- Metadata Suggestion tests should cover accept, edit then accept, dismiss, and Metadata Provenance retention without live AI.
- Vault Trash tests should cover move, exclusion from active views and search, retained references, restore, collision suffixes, no automatic purge, permanent-deletion confirmation, and known-reference cleanup.
- CLI tests should execute the binary and assert structured output and exit behavior for add-files, problems, trash, restore, and permanent deletion, following existing CLI integration-test patterns.
- Tauri command tests should use the real archive core behind the typed command contract and verify serialization, state management, error mapping, remembered Vaults, import progress, and workbench snapshots.
- Playwright tests should exercise the TypeScript workbench against controlled Tauri-command responses for first launch, create/open, remembered Vaults, import progress and summary, gallery browsing, search, editing, conflict handling, Review Reasons, Vault Problems, Vault Trash, restore, and error states.
- Playwright tests should include desktop and constrained viewport screenshots and verify that images render, text does not overlap, and dense workbench controls remain usable.
- The real Linux Tauri window will use a concise manual smoke checklist covering launch, native dialogs, file import, actual image rendering, edit/save, review resolution, trash/restore, close, and reopen.
- Automated native-window WebDriver testing is deferred. Revisit it after the Offline Archive Loop stabilizes and native integration regressions justify its maintenance cost.
- A generated-data benchmark should verify correct operation around 5,000 Saved Items and measure open, browse, search, and rebuild behavior. It should not impose premature optimization-specific assertions.
- Normal automated tests require no network, OpenAI credentials, or live external services.

## Out of Scope

- URL Capture, source extraction, cleaned-text fetching, or hostile-site fallback workflows.
- Live OpenAI calls, AI provider configuration UI, AI Budget Mode controls, or new AI enrichment runs.
- EXIF, XMP, or IPTC metadata extraction.
- Reverse image search, image identification, vector search, embeddings, or visual similarity search.
- Duplicate record merging.
- Collection creation, collection editing, or Item Link editing through the UI.
- Continuous filesystem watching or a required background service.
- A dedicated Activity Log browser.
- Automatic Vault Trash purging.
- Automatic folder renaming after metadata changes.
- Migrating disposable format-version-1 prototype Vaults.
- Supporting more than one Active Vault at a time.
- Automated native-window WebDriver testing in this milestone.
- AppImage, Flatpak, Snap, `.deb`, package signing, installers, or automatic updates.
- Verified Windows or macOS builds.
- A frontend component framework unless implementation evidence shows modular TypeScript is insufficient.
- A broad archive-core refactor unrelated to the Offline Archive Loop.
- Performance architecture for collections materially larger than the 5,000-item personal-use target without benchmark evidence.

## Further Notes

The acceptance demonstration is one uninterrupted personal workflow: launch the real Linux desktop window, create an empty Vault, add selected images, recursively import a folder containing nested images, observe exact-duplicate and failure handling, browse actual Thumbnail Previews, search, edit an Item Record, resolve every Review Reason, move and restore an item through Vault Trash, close the application, relaunch it, and confirm the same canonical Vault state remains available without network or AI access.

The existing archive-core behavior and tests are a foundation, not a compatibility constraint. Implementation should preserve the established domain decisions that Vault files are canonical, Preserved Files are untouched, derived state is rebuildable, every Saved Item has one Home Subvault, and stable IDs do not depend on readable Item Folder paths.

The first real dogfood Vault marks the beginning of on-disk compatibility responsibility. Before that point, format and APIs should be made coherent even when doing so requires replacing prototype behavior.
