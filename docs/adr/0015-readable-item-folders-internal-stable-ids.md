# Readable Item Folders, Internal Stable IDs

Item folders should use readable names rather than leading stable IDs, because the filesystem remains part of the user's recovery and inspection experience. Stable item IDs should live inside item records for internal references, collections, and indexes; folder-name collisions can be handled with a small suffix only when necessary. When enrichment improves metadata, the app may suggest a folder rename, but it should not silently move item folders without user approval.

For the Paintings subvault, the preferred readable folder pattern is `{Creator} - {Year} - {Title}`, with explicit unknown fallbacks when creator, year, or title cannot be inferred confidently.
