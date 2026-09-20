# Validation and known limits

## Latest checks

At `fd489b6`: 62 unit tests, 7 Pi worker tests (one optional private replay skipped), 12 rootless container tests and Chromium extension integration passed, along with typecheck. These exercise inherited policies/retries, export authorization and file integrity, context bounds, CONNECT broken-pipe recovery and completion after the final save. Deterministic provider fixtures are not live attribution benchmarks.

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

- **Transport errors:** proxy CONNECT rejection could terminate the controller with `EPIPE`; commit `066c7b2` adds handling and a deterministic regression. A separately reported `ECONNRESET` failure has not been independently reproduced. Do not treat every transport failure as resolved.
- **Context accumulation:** a YouTube retry saved a record then exceeded the model-input bound on another request. Duplicate previews and oversized public candidate lists are now bounded, local research is limited, and a successful final save needs no further model call. Subsequent local-image imports have also exceeded the 64,000-token input bound despite the early research cutoff; the batch continued past those unsuccessful records. These changes and paging still do not bound the whole conversation. Full preserved source and model context need separate treatment.
- **Interpretation quality:** sample checks found both supported identifications and uncertain or overconfident descriptions. Preserving a file successfully does not establish that its generated metadata is correct. File-integrity checks and model-quality review are separate. Local-image prompting remains artwork-specific; see [current specialization](files.md#attribution-and-current-specialization).
- **Browser extraction:** scripts/forms are stripped and post isolation is attempted, but completeness varies with site layout, viewport, transcript UI and virtualized content. No ongoing signed-in browser interaction capability exists.

## Other boundaries

- Rootless isolation has been tested on Linux with Podman/crun; this is not a full host-hardening audit. Public HTTPS and other container platforms need separate validation. Live OpenRouter compatibility has not been established by fixture tests.
- The worker prepares dimensions, previews and visual hashes; the controller validates structure, bytes and bounds rather than independently decoding all artifact metadata. A worker with permitted network access can disclose content it legitimately receives. Tests do not prove immunity to all prompt injection or kernel exploits.
- Hard crashes can leave receiver/writer/reclamation locks or staging files. Inspect ownership and running processes before recovery; never blindly remove a live lock. Publication is not guaranteed durable across every power-loss scenario.
- Arbitrary file rearrangement, general synchronization, automatic long-source compaction, unsupported image formats and an approval UI for the legacy decision queue remain outside current functionality.
