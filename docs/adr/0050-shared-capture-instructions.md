---
status: accepted
---

# Shared capture instructions in CAPTURE.md

Use one user-authored CAPTURE.md at the vault root for default content/detail preferences. Supply it to modern capture jobs as a read-only controller snapshot. Individual capture instructions (including retained source instructions) take precedence. Page content remains untrusted evidence and cannot edit the policy; the file does not grant additional tools or management permissions.

Initialize missing files with detailed defaults: enumerate substantive tips, retain actionable steps/examples/prerequisites/caveats, explain essays' supporting arguments, and avoid invention and filler. Preserve existing files, allow an empty policy, and limit content to 16,000 UTF-8 bytes. No nested policy hierarchy is introduced.

An authenticated receiver endpoint reads and writes only this fixed file. Browser capture pairings can edit these preferences without general vault-management scope. The extension settings editor uses a content revision to reject stale writes; direct file edits are read for the next job. Saves use the vault lock, atomic replacement and normal history. Symlinks and non-regular files are rejected. Direct user edits outside GG cannot participate in its write lock; revision checks protect observed earlier changes, not an arbitrary external editor racing the final replacement.

Policy changes affect future jobs and explicit recaptures, not automatic archive rewrites. Existing per-snapshot completed records and preservation of human edits remain unchanged.
