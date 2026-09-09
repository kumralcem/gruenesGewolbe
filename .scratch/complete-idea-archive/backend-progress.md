# Backend progress

Updated: 2026-09-07 (resumed backend pass)

## Implemented in the working tree

- Added a separate native Idea Source capture path. Pasted text is preserved immediately; URL extraction accepts readable text only and never turns an idea request into an artwork capture.
- Added bounded generic HTML extraction using a real HTML parser, preferring `article`, then `main`, then `body` content.
- Added offline `read_idea_source` and lightweight `get_item_details` commands.
- Added OpenAI provider configuration/status and Responses API summarization with explicit input/output/timeout bounds and source-first persistence.
- Added TypeScript contracts and Tauri adapter methods agreed with the UI agent.
- Hardened source-copy reads against absolute paths, traversal, symlink escapes, empty files, and files over 2 MiB; the same reader feeds UI and AI enrichment.
- Tightened the OpenAI configuration file to owner-only mode on Unix.

## Remaining

- Compile and run the focused bounded Rust tests after the resumed fixes, then inspect the final diff and report any remaining gaps.

## Resumed fixes now in the working tree

- Live summary application snapshots and rechecks the Active Vault root, Item Record revision, and exact preserved source text. The remote interval now has a test seam; tests switch to an identical copied Vault, edit source text, and edit the Item Record during a fake provider call.
- Automatic post-capture summarization carries the original Active Vault root into that same guard.
- Provider failure has a regression test proving the preserved source remains readable and no summary is written.
- Providers can declare an unknown cost estimate; the live completion adapter does so, preventing a paid call with unknown billing from being logged as a zero-cost call.
- Pasted or extracted Idea Source text over the 2 MiB storage/read limit is rejected before an item folder is created.
- Generic page redirects and supported-media redirects remain separate. Supported redirects stay in the originating Wikimedia, X-page, or X-media host family; redirect destinations must also be HTTPS, credential-free, and not literal local/private addresses.
- Literal bracketed IPv6 and IPv4-mapped IPv6 local/private addresses are rejected. DNS names resolving to private addresses are not currently pinned or resolver-validated.
- X login/interstitial metadata is rejected as source content.
- Wikimedia-derived file names percent-decode their final path segment.

## Coordination note

- The formatting-only Rust test changes listed in `PROGRESS.md` are safe to restore, but this agent's sandbox cannot write `.git/index.lock`; root must perform that restore.

## Constraints

- Preserve the pre-existing uncommitted image extractor work in `apps/desktop/src/source_extractor.rs`.
- Do not issue a live paid OpenAI request; provider behavior is verified with local/fake responses.
- Root agent owns final commits and push.

## Verification log

- `bash scripts/check-bounded.sh cargo check -p gruenes-gewolbe-desktop --features tauri-runtime`: passed after enabling reqwest's JSON feature; warm run peak 291.9 MiB. Initial run found the missing reqwest JSON feature and otherwise reached the desktop crate.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-core --test capture`: 9 passed, peak 500.2 MiB. Includes offline reopen/read, traversal, empty-copy, and symlink-escape coverage.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --features tauri-runtime --lib`: 13 passed, 1 failed, peak 1.9 GiB. New HTML extraction tests passed. The remaining failure is the pre-existing Wikimedia filename assertion (`%28`/`%29` need decoding to parentheses).

### Resumed verification

- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-core --test capture`: 10 passed, peak 353.6 MiB, zero swap. Covers offline source reads, traversal/symlink/empty/oversize reads, and rejecting oversize pasted text before item creation.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --features tauri-runtime --lib`: 16 passed, peak 1.3 GiB, zero swap. Covers generic HTML, X metadata rejection, redirect boundaries, private literal addresses, and decoded Wikimedia filenames. A final pure X-login/media assertion was added afterward; root's final all-features workspace run will cover it.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --features tauri-runtime --bin gruenes-gewolbe`: 5 passed, peak exactly 2 GiB, zero swap. Covers identical copied-Vault switching, source changes, Item Record revision changes, provider failure preservation, and media protocol containment.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-core --test ai_enrichment`: 7 passed, peak 293.2 MiB, zero swap. Covers unknown provider cost not being recorded as a free call.
- `bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --test tauri_commands --test ai_provider_config`: 13 passed, peak 483.5 MiB, zero swap. Covers command contracts, source-first capture/summarization, active-Vault capture guard, and Unix config mode `0600`.
- `bash scripts/check-bounded.sh cargo check -p gruenes-gewolbe-desktop --features tauri-runtime`: passed, peak 823.5 MiB, zero swap. This preceded the final pure extractor assertion/`cfg(test)` annotations; root owns the final all-features compile/test.
- `rustfmt` was scoped to the backend files changed by this slice. `git diff --check` passes. No paid or live provider request was issued.

## Review resolutions and remaining limitations

- Resolved the high redirect-boundary finding in `backend-review.md`: supported-source redirects stay within their originating Wikimedia page, X page, or X media host family; generic Idea Source redirects use a distinct public-HTTPS policy.
- Literal loopback, private, link-local, unspecified, bracketed IPv6, and IPv4-mapped IPv6 addresses are rejected. DNS hostnames are not resolved and pinned before requests, so DNS rebinding/private DNS resolution remains a network-boundary limitation.
- Generic HTML and provider responses fail when their byte limits are exceeded rather than silently truncating. The Wikimedia markup candidate scan uses bounded sections and may fall back if media links occur unusually far from the recognized section marker; it does not save partial text.
- X login/interstitial metadata is rejected for both readable text and media extraction.
- Live OpenAI billing is reported as unknown and therefore omitted from the numeric cost log; exact provider usage/cost accounting is not implemented.
- Thumbnail preparation still runs while the global desktop state mutex is held. Removing that contention safely needs a separate state/write-serialization design; this pass did not unlock canonical Vault writes or weaken mutation ordering.

## Final integration validation, 2026-09-08

All final automated checks passed; see [VALIDATION.md](VALIDATION.md). Both navigation baselines and capture/reader screenshots were inspected. Earlier pending automated checks in this historical log are superseded by that result. Native smoke and measured native responsiveness remain outstanding.
