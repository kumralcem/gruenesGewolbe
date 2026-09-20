# Capture context failures and keyboard submission

Both reported jobs failed with `Model input exceeds configured bound`. The transcript job failed after browse/read_snapshot; the artwork job accumulated research tool outputs, including a failed fetch and raw HTML. Source snapshots were present; this was not loss of pairing or missing browser access.

A deterministic real-Pi fixture reproduced the transcript failure at the same tool boundary. Fixture model calls previously bypassed the production context bound; they now enforce the same estimate so oversized tool responses fail in tests. Fixed automatic thumbnail previews for transcript browsing, duplicate initial text/HTML, unbounded fetch_text output, and exact-string source matching for common YouTube/X aliases. Source preservation remains independent of model-facing excerpts. Bounds were not raised and no live model calls were used for verification.

Stored user snapshots were replayed locally with a fixture model; private input data was not added to the repository. The optional GG_REPLAY_INPUTS array of local input.json paths supports future reproductions. This checks transport/context handling, not live-model interpretation or successful public attribution research.

Enter submits the per-capture instructions field; Shift+Enter inserts a line. IME composition, held-key repeats, and disabled submission are guarded. The shared CAPTURE.md editor retains ordinary multiline editing.

Prevention: enforce production context budgets in fixture tests and bound model-facing tool payloads independently of preserved source size. Long-context compaction remains separate work; repeated or very large reads can still reach the configured limit.
