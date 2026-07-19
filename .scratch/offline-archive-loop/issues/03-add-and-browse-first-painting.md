Status: ready-for-agent

# Add and Browse the First Painting

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build the first complete Saved Item path in the real workbench: select one or more local images, preserve them in the Paintings Subvault, write format-version-2 Markdown Item Records, generate actual Thumbnail Previews, browse them in the gallery, and inspect the Preserved File and record details. This slice establishes the coherent record codec and preview behavior used by later workflows.

## Acceptance criteria

- [ ] A native file dialog accepts one or several JPEG, PNG, WebP, or GIF files and adds each as a Paintings Saved Item.
- [ ] Each selected file is copied without changing its original bytes or format and receives one Home Subvault, readable Item Folder, and collision-resistant stable ID.
- [ ] Each Item Record is one Markdown document with parsed structured frontmatter and readable body sections rather than ad hoc line handling.
- [ ] Initial metadata uses filename parsing, basic file facts, and optional user input while ignoring EXIF, XMP, and IPTC metadata.
- [ ] The app generates bounded, rebuildable Thumbnail Previews in derived state and uses the first GIF frame initially.
- [ ] A preview decoding failure still preserves the file, shows a gallery placeholder, creates a Review Reason, and records an Activity Log event.
- [ ] The gallery shows actual previews, defaults to newest first, and supports title, creator, year, oldest, and newest sorting.
- [ ] Selecting a gallery item opens details for the Preserved File and common Item Record fields.
- [ ] The CLI can add selected files and returns structured results without driving the GUI.
- [ ] Tests use real decodable fixtures and cover original preservation, record parsing, preview bounds, GIF behavior, preview rebuild, decoding failure, gallery selection, sorting, and CLI output.

## Blocked by

- .scratch/offline-archive-loop/issues/01-launch-and-reopen-format-v2-vault.md

