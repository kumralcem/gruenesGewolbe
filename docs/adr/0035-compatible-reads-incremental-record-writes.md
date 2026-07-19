---
status: superseded by ADR-0038
---

# Compatible Reads and Incremental Record Writes

New application versions should read older supported item-record shapes without rewriting the whole vault on open, deriving newer concepts such as Review Reasons when fields are absent. Richer record fields are written when the affected item is edited or reviewed; genuinely incompatible vault-format migrations require an explicit backup and user confirmation.
