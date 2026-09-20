---
status: accepted
---

# Discover nested destination folders

User decision on 2026-09-20: the existing folder hierarchy should guide automatic capture routing. This refines ADR-0047: destinations are no longer restricted to a registered flat list.

Keep the current `subvaults/` container for compatibility. Discover ordinary directories beneath it recursively before reading or writing the vault and before starting an agent job. Paths such as `Photography/Historic` are destination identities. A parent may hold records in its own `items/` folder and also have nested destination folders. Manually creating an empty folder is sufficient; GG creates its `items/` directory when first saving there. The marker's `areas` list is a compatibility/display-order hint, not the authority for which destinations exist.

Do not recurse into hidden folders or reserved `items` record trees, and do not follow symlinks. Destination paths have at most 16 components, 100 characters per component and 500 total characters. No daemon, periodic watcher, automatic file import or registration step is required.

The capture agent sees discovered relative paths and chooses the most specific fitting destination, inspecting previous GG records through search/read when useful. Uncertainty still goes to Inbox. Explicit management accepts nested paths under an existing parent; parent renames update descendant metadata. History now records directories as well as files so undo can restore empty folders and reject later additions. Older file-only history remains readable and ordinary file undo remains supported. Legacy create/rename folder undo is rejected with a manual recovery message because those journals lack empty-directory ownership information; existing pending confirmation proposals may need regeneration because directory-sensitive fingerprints differ.

Tracking arbitrary external rearrangements is investigated but deferred. Whole-record moves could be detected using stable IDs in `record.md`; resolving duplicate IDs, repairing relative links when individual assets move, and preserving undo semantics require a separate reconciliation policy. Files remain authoritative; any future path inventory should be derived state rather than a competing vault source of truth.
