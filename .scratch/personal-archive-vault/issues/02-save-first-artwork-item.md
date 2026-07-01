Status: ready-for-agent

# Save a First Artwork Saved Item

## Parent

.scratch/personal-archive-vault/PRD.md

## What to build

Build the narrowest complete path for adding one artwork saved item to a visual subvault from a local image file. The workflow should create an item folder, preserve the original media file, write a Markdown item record, assign a stable internal item ID, set a home subvault, and make the saved item visible in the desktop app.

This is the tracer bullet for the saved item model. It should prove that ordinary files on disk are the durable archive record and that the app can read the same record back.

## Acceptance criteria

- [ ] A user can add a local image file to a Paintings subvault from the app.
- [ ] The original file format is preserved and the saved file is not silently converted or replaced.
- [ ] The saved item has one home subvault and one readable item folder.
- [ ] The item record includes the stable item ID, source/import provenance when available, saving reason when supplied, review status, and basic descriptive fields.
- [ ] The saved item can be reopened from disk after the app restarts.
- [ ] Tests assert on the created item folder, preserved file, and item record contents through the archive core public API.

## Blocked by

- .scratch/personal-archive-vault/issues/01-create-and-open-active-vault.md
