Status: ready-for-agent

# Resolve Duplicate Candidates Without Merging

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Complete the ambiguous duplicate review path without introducing record merging. From the Duplicate Candidate Review Reason, the user should be able to declare that the items are unrelated, retain both intentionally, or recoverably remove the current item through Vault Trash.

## Acceptance criteria

- [ ] A Duplicate Candidate Review Reason shows the candidate item, matching evidence, and enough details to compare the Saved Items.
- [ ] Not a Duplicate records the user's decision and resolves the reason without changing either Item Folder.
- [ ] Keep Both records an intentional-overlap decision and resolves the reason without changing either Item Folder.
- [ ] Move This Item to Vault Trash uses the standard recoverable-removal workflow and keeps the other Saved Item active.
- [ ] Each resolution updates derived Review Status consistently and removes resolved work from the Review Queue.
- [ ] Exact File Duplicate import skipping remains separate from ambiguous Duplicate Candidate resolution.
- [ ] No merge-files or merge-records operation is introduced.
- [ ] Tests cover all three actions, evidence display, Review Status transitions, restart, and interactions with Vault Trash.

## Blocked by

- .scratch/offline-archive-loop/issues/05-skip-exact-file-duplicates-during-import.md
- .scratch/offline-archive-loop/issues/07-resolve-review-reasons-individually.md
- .scratch/offline-archive-loop/issues/09-move-items-to-vault-trash-and-restore.md
