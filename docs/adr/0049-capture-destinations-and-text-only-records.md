---
status: accepted
---

# Requested capture destinations and text-only records

A capture may create destinations explicitly requested in the user's capture instructions. A narrow create_destination tool accepts a relative folder path and creates it through existing vault validation and history. Parents must exist; the agent can create them in order. The controller requires user instructions, permits only creation through this endpoint, and returns existing destinations unchanged. Capture does not gain general management, rename, delete or undo capabilities. Page content remains evidence, never authorization. Determining whether natural-language instructions request creation remains an agent responsibility.

The capture tool supports includeImages=false for idea/instruction records when images are irrelevant or explicitly excluded. The controller requires an idea with no image assets or image errors for that mode. It still preserves source text and reports source truncation. Other captures retain conservative missing-image reporting by default; visual records cannot opt out. Aggregate job status follows persisted record outcomes so a discarded decorative image does not independently make a complete text record partial. Collection can still download candidate images before the agent classifies their relevance.

Recaptures retain existing folder placement and preserved earlier assets. Moving existing records and deleting earlier assets remain explicit management work. A new text-only capture is not authorization to erase previously preserved files.
