---
status: accepted
---

# Agent-Classified Visual Capture

The user chooses Visual Capture rather than Idea Source capture, and the agent determines the kind of visual work and chooses among existing home subvaults, such as Paintings, Photography, or Sculptures. Sources are open-ended, with X, Wikipedia, and Wikimedia Commons as representative starting points rather than an exhaustive supported-host list.

Vault and subvault creation and deletion belong exclusively to the user, including setup defaults. The capture agent cannot create or delete them; choosing a destination does not confer authority to change the available areas. If no existing subvault fits, flag the capture in a persistent Capture Queue so the user can later approve a destination or give the agent further instructions. The queue's storage layout and interaction commands remain open.

The descriptive record includes creator, title, year, style, subjects, mood, tags, source links, and a short visual description where applicable. Factual attribution is distinguished from interpretation, and an unidentified image is saved automatically with uncertain facts left explicitly unresolved and reported to the user. The exact record schema remains open; [ADR-0042](0042-one-selected-image-per-visual-capture.md) defines image selection and research boundaries.
