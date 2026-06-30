# Visible Vault Files, Hidden Derived State

The vault layout should separate canonical archive files from rebuildable app state. Human-readable vault configuration, tag registry, collections, subvaults, item folders, item records, and preserved files should be visible ordinary files, while derived indexes, embeddings, cached thumbnails, and caches should live under a hidden app folder such as `.gruenesgewolbe/` and remain rebuildable from the visible vault. Thumbnails should be cached rather than regenerated for every gallery view, but they are not canonical archive content.
