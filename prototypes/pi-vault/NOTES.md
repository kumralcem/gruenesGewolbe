# Prototype findings — 2026-09-10

**Pi is a workable integration foundation without a fork. Public source access is the main unresolved product constraint.** The sandbox/controller split, real streaming model calls, file captures, bounded image preparation, and natural-language rediscovery all ran successfully. These results justify continuing the CLI implementation, but they do not establish that a public-only client can capture the user's mostly-X workload reliably.

The branch is `codex/pi-vault-prototype`. The old application and live archive remain intact. The prototype is deliberately retained with tests so the user can inspect the behavior before repository replacement.

## Live source results

The runs used the existing application's configured OpenAI model, `gpt-5.6-luna`, through the Responses API. This is a test result, not a hardcoded model choice. Initial Chat Completions returned HTTP 400; the configurable API format resolved it. No provider key entered the worker environment, prompts, mounted files, or reports.

| Input | Observed outcome |
| --- | --- |
| Commons: Le Chevalier aux Fleurs | Saved the selected painting, metadata, original bytes, and preview in Paintings. 31.8 seconds in the recorded run. |
| French Wikipedia: selected Cabanel atelier media | Honored the selected image fragment and saved the atelier painting in Paintings. 28.0 seconds. Preview visually inspected. |
| X: solisolsoli | HTTP 403; skipped without an item or queue entry. |
| X: EvolveWildlife | HTTP 403; skipped without an item or queue entry. |
| X: Noldorcitizen | HTTP 403; skipped without an item or queue entry. |
| Supplied YouTube video | Page loaded, but the worker did not retrieve a transcript; skipped. This is an extraction/access limitation, **not proof that the video has no captions**. Earlier interactive browser inspection did retrieve captions. |
| Wikipedia: Inbox Zero | Saved a concept summary and source text in Ideas. 15.3 seconds in the first run. |
| Customer/email-agent retrieval | Returned exactly the three relevant synthetic notes, with existing file paths and specific relevance explanations; excluded the art and gardening notes. 11.2 seconds. |
| Unrelated reactor-pump retrieval | Returned no files. 10.3 seconds. |

This is a small smoke evaluation, not a retrieval benchmark. The three relevant notes were deliberately written with varied vocabulary; larger archives, languages, paraphrases, and distractor-heavy queries remain to be evaluated. Literal search has no embedding index. `gg ask` successfully expanded and checked candidate records in these examples.

Later live Commons and retrieval rechecks each hit their 120-second deadlines during model requests. The controller reported failure and stopped both jobs. A subsequent small diagnostic completed through the gateway in 2.15 seconds (HTTP 200), while the simultaneous direct provider request timed out after 30 seconds. This shows the latest gateway can still stream live responses and the stalls also occur outside it; the exact provider/network cause remains unresolved.

The two saved paintings used accessible 1280×815 and 960×763 copies. Attempts to obtain larger files did not establish a usable improvement. The prototype does not promise the maximum resolution available anywhere on the internet.

## Image compatibility

These files were read from the existing archive as fixtures, processed in fresh containers, and saved only into disposable test vaults. SHA-256 checks verified both the archived input and preserved copy against the original bytes.

| Fixture | Original | Preview |
| --- | --- | --- |
| Cavalcade WebP | 8000×8000; 7,153,234 bytes | 768×768; 201,032 bytes |
| Gérôme Carpets JPEG | 4946×6326; 6,255,293 bytes | 600×768; 80,202 bytes |
| Friedrich Wanderer JPEG | 5256×6742; 31,235,405 bytes | 599×768; 59,149 bytes |

All passed. This rules out a blanket inability to process WebP or these large JPEGs in this pipeline. It does not diagnose the old application's failure. Animated images, TIFF, HEIC, corrupt files, and images beyond the configured limits have not been established as supported.

## Automated verification and review

13 unit tests and 8 real-container integration tests pass, along with TypeScript checking and formatting. The real pasted-list CLI also returned failed → skipped → saved for an invalid URL, blocked source, and valid source, with the expected nonzero batch exit status. The no-key demo was executed successfully. [Review findings and their fixes](reports/review.md) are retained separately.

## Isolation and lifecycle evidence

The real worker ran as the user's non-root UID with no direct network and no host home, vault, real API key, or container-runtime socket mounted. Its read-only application filesystem rejected writes, temporary scratch accepted them, and Chromium's own sandbox remained enabled. The gateway rejected loopback/private/reserved targets, unauthorized requests, unexpected models, extra model calls, writes from `ask`, invalid destinations, and competing outcomes for one input. A deliberately hung provider job hit its deadline; the next capture succeeded.

On this host, a shared Unix socket failed across the rootless namespace boundary. The implementation instead inherits one private descriptor through Podman/crun. A framed channel multiplexes traffic only to that job's host gateway. No nested container runtime or daemon socket is required. The rootless browser sandbox required `SYS_CHROOT`; all other extra Linux capabilities remain dropped.

These checks cover concrete boundaries, not every kernel exploit, race, or prompt-injection strategy. A network-capable capture worker can disclose source/archive text it legitimately receives. Model use is limited by requests, output tokens, time, and bytes; it is not an exact currency cap.

## Remaining implementation issues

1. **Public access:** all three supplied X posts were blocked; YouTube transcript retrieval needs a focused public-session investigation. Keep skip-and-report behavior. No login, cookies, audio download, or transcription fallback was added.
2. **OpenRouter live compatibility:** real Pi selected the OpenRouter provider and exercised tool-call streaming against a deterministic upstream stub. No OpenRouter key was available for a live-provider check. Run `GG_PROVIDER=openrouter GG_MODEL=YOUR_SLUG pnpm evaluate:live` with `OPENROUTER_API_KEY` configured before calling that path live-validated.
3. **Image identity and quality:** deduplication uses a selected-image URL or exact content hash. Upgrades require identical normalized 32×32 pixel hashes and increasing dimensions. This is intentionally conservative and often rejects recompressed/resized equivalents; it is not a perceptual similarity classifier. Different URLs/bytes for the same art can remain duplicate items. Width, height, fingerprint, and preview are prepared by worker code; controller checks are structural and byte/hash bounds, not independent trusted decoding. A stronger artifact-integrity boundary is needed before production use against actively malicious worker code.
4. **Capture fidelity:** image selection, attribution, routing, and summary meaning remain model judgments. The prototype tests plumbing and a few live examples; it does not establish reliable photography/sculpture classification from the blocked X examples. Idea source text is captured by the browse tool independently of the model's summary or focus. Explicitly included linked pages retain URL headers, with a combined 400,000-character ceiling. Source reads support bounded continuation. Arbitrarily long sources require chunking later.
5. **Durability and interface:** new-item publication uses an atomic rename, but there is no fsync-level crash guarantee. A hard crash can leave a write lock/staged or unreferenced files requiring inspection. Only one controller writer is supported. The prototype lists queued decisions; it does not yet offer queue resolution, a settings wizard, live-archive import, repository replacement, or a hosted GUI.
6. **Intermittent model requests:** later live jobs stalled despite earlier successful captures/retrieval. The direct-versus-gateway diagnostic above confirms a failure can occur outside the worker/gateway. Keep deadlines and clear errors; investigate provider/network behavior before regular use.
7. **Portability:** tested on this Linux rootless Podman/crun setup. Docker, macOS, Windows, and hosted-IP source access remain untested. Browser proxying currently supports HTTPS; the explicit fetch tool also supports public HTTP.

## Decision

Continue with Pi's SDK and the controller-owned file archive. Keep `gg search` as a cheap deterministic tool and `gg ask` as optional agent-assisted retrieval. Resolve the X/YouTube access limitations and independently validate artifact metadata before treating the archive agent as ready for everyday use. Then implement the clean CLI's settings/queue workflows; build the private browser service on the same operations afterward.
