Status: ready-for-agent

# Add and Browse the First Artwork Saved Item

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the first complete Saved Item path in the real workbench: select one or more local images, preserve them in the Paintings Subvault, write format-version-2 Markdown Item Records, generate actual Thumbnail Previews, browse them in the gallery, and inspect the Preserved File and record details. This slice establishes the coherent record codec and preview behavior used by later workflows.

## Acceptance criteria

- [x] A native file dialog accepts one or several JPEG, PNG, WebP, or GIF files and adds each as an Artwork Saved Item in the Paintings Subvault.
- [x] Each selected file is copied without changing its original bytes or format and receives one Home Subvault, readable Item Folder, and collision-resistant stable ID.
- [x] Each Item Record is one Markdown document with parsed structured frontmatter and readable body sections rather than ad hoc line handling.
- [x] Initial metadata uses filename parsing, basic file facts, and optional user input while ignoring EXIF, XMP, and IPTC metadata.
- [x] The app generates bounded, rebuildable Thumbnail Previews in derived state and uses the first GIF frame initially.
- [x] A preview decoding failure still preserves the file, shows a gallery placeholder, creates a Review Reason, and records an Activity Log event.
- [x] The gallery shows actual previews, defaults to newest first, and supports title, creator, year, oldest, and newest sorting.
- [x] Selecting a gallery item opens details for the Preserved File and common Item Record fields.
- [x] The CLI can add selected files and returns structured results without driving the GUI.
- [x] Tests use real decodable fixtures and cover original preservation, record parsing, preview bounds, GIF behavior, preview rebuild, decoding failure, gallery selection, sorting, and CLI output.

## Blocked by

- .scratch/offline-archive-loop/issues/01-launch-and-reopen-format-v2-vault.md

## Comments

Implemented across archive-core, the CLI, Tauri commands, and the browser workbench. Selected JPEG, PNG, WebP, and GIF files are byte-preserved under Paintings with UUID-backed records, optional batch metadata, source-preserving structured YAML updates, rebuildable bounded PNG previews, first-frame GIF handling, and review-safe placeholders for decode failures. The gallery uses real local asset URLs, metadata sorting, and Primary File details. Evidence includes the real-image core fixtures, CLI integration test, Tauri command coverage, browser gallery flow, a successful Tauri release build, and `cargo test --workspace --all-features`.
