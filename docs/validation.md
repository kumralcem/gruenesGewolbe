# Validation and known limits

## Latest checks

Before this iteration, commit `84d94c6` passed typecheck, **54 unit tests**, **5 real Pi worker tests** (optional private replay skipped), and **12 rootless container tests**. The current iteration passed 61 unit tests plus the added attribution-tag regression, 6 Pi worker tests (one optional private replay skipped), 12 rootless container tests and the Chromium extension integration (run inside the Playwright image). Typecheck also passed. Current changes exercise inherited policies/retries, portable export authorization and bytes, duplicate-image context reduction, CONNECT broken-pipe recovery and no completion after final save. These use deterministic provider fixtures; they are not live attribution benchmarks. Earlier reviews and run transcripts remain in Git history.

Run checks from the checkout:

```sh
pnpm typecheck
pnpm test
pnpm test:worker
pnpm build:worker
pnpm test:container
pnpm test:extension
```

The extension suite requires Chromium's OS dependencies; the Playwright worker image can supply them when absent on the host. The first rootless launch after an image build may be substantially slower than warm launches. `gg probe` checks worker/browser isolation without model usage. Optional `GG_REPLAY_INPUTS` replay in `test/worker.integration.ts` reads local snapshot paths; do not commit private snapshots.

Coverage includes credential/network/filesystem isolation, bounded model requests, cancellation, original-image preservation, source retention, nested destinations, undo conflicts, partial retry plans, pairing/revocation, usage controls and attribution evidence. The attribution test verifies that claims cite fetched evidence; matching evidence to the pictured artwork remains model judgment.

## Live findings requiring follow-up

- **Transport crash:** the collection import stopped at item 10 with unhandled `write EPIPE` in the proxy CONNECT rejection response. Commit `066c7b2` fixes it with a deterministic regression; the resumed live import passed that item. An earlier browser job crashed with `read ECONNRESET`; that exact trace has not been independently reproduced. See [handoff](handoff.md) for current service/log details. Do not call unattended operation reliable yet.
- **Context accumulation:** a YouTube retry saved a record then exceeded the model-input bound on another request. Duplicate previews and oversized public candidate lists are now bounded, local research is limited, and a successful final save needs no further model call. These changes and paging still do not bound the whole conversation. Full preserved source and model context need separate treatment.
- **Attribution research:** two new live imports saved with honest provisional metadata after research failures. Routing improved, but automatic identification is not reliably verified. Seven earlier records received a separate manual evidence/routing review.
- **Browser extraction:** scripts/forms are stripped and post isolation is attempted, but completeness varies with site layout, viewport, transcript UI and virtualized content. No ongoing signed-in browser interaction capability exists.

## Other boundaries

- Rootless isolation passed on this Linux/Podman/crun host; this is not a full host-hardening audit. Public HTTPS and other container platforms need separate validation. Live OpenRouter compatibility has not been established by fixture tests.
- The worker prepares dimensions, previews and visual hashes; the controller validates structure, bytes and bounds rather than independently decoding all artifact metadata. A worker with permitted network access can disclose content it legitimately receives. Tests do not prove immunity to all prompt injection or kernel exploits.
- Hard crashes can leave receiver/writer/reclamation locks or staging files. Inspect ownership and running processes before recovery; never blindly remove a live lock. Publication is not guaranteed durable across every power-loss scenario.
- Arbitrary file rearrangement, general synchronization, automatic long-source compaction, unsupported image formats and an approval UI for the legacy decision queue remain outside current functionality.
