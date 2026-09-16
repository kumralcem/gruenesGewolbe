# Implementation review — 2026-09-16

Fixed point: `1aeb3714ec396c93192f3a30436302de4f5f0b23`, the pre-implementation HEAD. Reviewed staged changes before final commit with two independent agents under the review skill. Spec: `PRD.md`. Standards: repository AGENTS/domain docs, accepted ADR-0047 and the skill's heuristic baseline. No extra style-only refactors requested.

## Standards

No hard violations of documented agent workflow/domain vocabulary found. Four correctness findings:

1. **High — stale lock reclamation can defeat serialization.** Two contenders could observe a dead PID and unlink each other's replacement. Fixed with serialized reclamation and an owner recheck; a six-process regression verifies no overlap. Waiting respects cancellation. A crash holding the reclamation gate requires documented manual recovery.
2. **High — cached subvault lists can overwrite newer state.** Separate controller instances could overwrite newly registered destinations. Fixed by reading the marker under the writer lock and before discovery; tested with two instances creating different destinations.
3. **Medium — temporary refresh records are published as real items.** A crash could leave a discoverable duplicate. Proposals now stay in hidden `.staging/proposal-*`, outside normal item discovery.
4. **Medium — recapture overwrites a manually edited heading.** H1 updates now compare the actual heading with the generated baseline; human changes are preserved and flagged. Regression passes.

Reviewer recheck: all four code fixes sound. It caught a missing test import, since fixed with the affected suite passing. The manual recovery limitation for a stale `.reclaim` gate is documented.

## Spec

Three findings, no concrete scope creep:

1. **High — retry loses the original multi-record plan.** Requirement: “save successful records immediately and retry only unfinished work.” Plans, completed keys/outcomes and batch identity now persist per snapshot. A retry cannot shorten or rekey the plan; completed work is exposed to the worker and does not need regenerating. Follow-up review caught reliance on only the latest record baseline: this was also fixed with durable per-snapshot completion markers. Regression covers S1 partial save, S2 recapture updating the saved record, then S1 retry; newer content survives and only unfinished work remains.
2. **Medium — CLI provides no proactive approaching-limit warning.** Requirement: “Warn at 80%” and “Show usage warnings in the extension and CLI.” Model job results now include usage warnings, current provider and reported allowance (or unavailable), so ordinary CLI/chat output exposes them automatically.
3. **Medium — capture tool instructions contradict the accepted workflow.** Requirement: “Multiple records and images.” Modern capture instructions now explicitly allow multiple supplied images and require continuing until all pending planned records are saved. Legacy single-image instructions apply only to legacy commands.

Reviewer final recheck: no remaining findings from these fixes. Host runtime/public endpoint limitations match the agreed deployment decision.

Summary: Standards 4 findings, all addressed (highest impact: lock/subvault integrity); Spec 3 findings, all addressed (highest impact: retry plan/completion preservation). Live provider and actual host-isolation checks remain operational follow-up, not passing fixture claims.
