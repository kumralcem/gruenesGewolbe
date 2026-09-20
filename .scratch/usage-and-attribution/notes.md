# Usage controls and artwork factuality

User authorized implementation, commit/push, then a collection allowance and live import. README documents commands/semantics. New temporary handoff section discusses future upload/download options; no UI/download feature authorized yet.

Evaluation: five images inspected visually; all seven original assets matched staging hashes. Useful descriptions but inconsistent Art vs Art/Paintings routing. Prior metadata reflected filenames without independent verification. Source spot checks support JOB (1896, Mucha Foundation), Horse and Train (1954, Art Gallery of Hamilton), Pacific (1967, Art Canada Institute). Zorn filename 1930 conflicts with published Studio Idyll/Ateljéidyll dates (1918); not silently reliable. Avoid blanket claims that the collection's artwork facts are verified.

New attribution stores evidence status per title/creator/year. The gateway checks that cited excerpts and claimed values occur in pages fetched in the current job. This prevents invented/unread citations but does not establish semantic correctness or source authority automatically. A model still has to match the artwork. Local filenames never automatically become verified artist/year fields.

Budget controls use per-window epochs to reset enforcement while retaining history. Collection grants are explicit hash allowlists with request/token/expiry caps; no provider failover, provider-pause bypass, automatic renewal or unlimited mode. Requests made under grants remain visible in rolling/history accounting. Successful calls refund over-reservation to the grant; failed calls retain reservations.

Validation: 54 unit tests, real Pi worker integration (5 passes, optional replay skipped), and all 12 isolated-container tests passed; typecheck and changed-file formatting passed. A targeted real Pi test fetched fixture evidence and persisted source-supported artist/year with exact original image bytes. No live model usage during tests.

The seven pre-existing imported records were updated through Vault.capture/history and appropriate move operations in one undoable batch. Obvious paintings formerly under Art moved to Art/Paintings; raw originals retained. Supported spot-check facts cited in records; unresolved filename claims remain explicitly unverified/uncertain. This is not a claim that all collection metadata is independently verified.
