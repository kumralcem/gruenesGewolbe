# Focused site captures — investigation only

User explicitly requested proposals, not capture changes, on 2026-09-20.

Observed from persisted job state and systemd logs:
- YouTube retry saved a record under Ideas, then failed another model request with `Model input exceeds configured bound`. Status is partial, not total loss. Tool trace: browse, read_snapshot twice, plan_capture, capture, saved, error. Reducing initial context did not solve cumulative conversation size.
- X job's running status was stale. Service crashed at 03:42:08 UTC with unhandled Socket `read ECONNRESET`; no worker or listener remained. The receiver's dead-owner lock caused repeated failed restarts. Verified dead PID, archived lock, restarted service; recovery marks job interrupted. No socket bug fix implemented and exact socket source not reproduced.
- Existing extractor already strips scripts/forms and attempts to isolate the matching X article. Full JavaScript functions are not deliberately fed to the model. Prior X snapshot was small, so large HTML is not established as its failure cause.

Proposed follow-up:
1. Typed X extraction: exact post ID, text, author, timestamp, attached media, explicit quoted-post context. Exclude replies, navigation, recommendation feeds. Report extraction uncertainty instead of silently treating the entire page as the post.
2. Typed YouTube extraction: title/channel/URL plus available transcript, no unrelated page content or thumbnails by default. Keep no-audio-transcription policy. Report missing/incomplete transcripts honestly.
3. Keep full source outside model context; bounded per-section transcript summarization followed by synthesis, retaining steps and timestamp references. Paging alone is insufficient when previous pages remain in conversation.
4. Context budgeting must include tool results and generated summaries; avoid a required extra completion just to acknowledge an already-completed save.
5. Separately reproduce socket-reset crash, contain transport failures to a job, and test dead receiver recovery. Do not merely increase memory or context caps.
