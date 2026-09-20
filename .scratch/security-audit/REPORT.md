# Security audit — Grünes Gewölbe

**Historical baseline:** the seven findings below have subsequent code fixes documented in [Remediation](REMEDIATION.md). Evidence and the original reproduction script describe the pre-fix commit.

Date: 2026-09-20. Audited commit: `dce41257f9443490fbc6fc46a9f80d4c5f0f5490`.

The project has substantial security engineering already, but there are unresolved vulnerabilities and trust-boundary weaknesses. Address the image decoder and archive/egress permissions first. Capture pairing should not currently be treated as a narrow, write-only capability. Using SSH forwarding instead of public HTTPS reduces exposure but does not resolve attacks delivered through captured content.

This audit adds documentation and harmless reproductions only. Application code, dependencies, credentials and deployment settings have not been changed. Severity below reflects this personal, single-controller application; it is not a claim of demonstrated host compromise. No critical application vulnerability or container escape was demonstrated.

## Scope and method

Reviewed all application modules under `src/`, extension JavaScript and manifest, package/lockfile, container configuration, installation scripts, service/proxy configuration, architecture and deployment guidance, and relevant tests. Examined authentication, authorization, source ingestion, model capabilities, SSRF, filesystem boundaries, exports, history/undo, credential handling, resource limits and dependency exposure.

Threat actors considered: a malicious captured page/document/image; a compromised capture-paired device; a compromised worker; an unauthenticated receiver client; and another local OS account where filesystem permissions permit access. A management-paired device is intentionally highly trusted. This is a personal vault, not a tenant-isolated service.

Tests used synthetic data and temporary vaults. Network exfiltration reproductions intercepted outbound operations; no archive data was sent to an external collector and no paid model calls were made. Direct gateway reproductions establish what a hostile worker or tool sequence can do, not the success rate of an indirect prompt injection against a live model.

## Findings

| ID | Severity | Finding | Evidence |
|---|---|---|---|
| SEC-01 | High | Vulnerable native image dependencies; MIME labels do not prevent affected decoding | Registry advisories + benign GIF decoding reproduction |
| SEC-02 | High | Capture combines unrelated archive reads with unrestricted public egress | Gateway reproduction; documented architectural risk |
| SEC-03 | Medium | Capture devices can change persistent vault-wide policy | HTTP reproduction |
| SEC-04 | Medium | Capture job access and cancellation have no device ownership boundary | Two-device HTTP reproduction |
| SEC-05 | Medium | Management gateway permits CONNECT despite intended network prohibition | Intercepted socket reproduction |
| SEC-06 | Medium, deployment-dependent | CLI-created archive files can be readable by other local users | File-mode reproduction with umask 022 |
| SEC-07 | Low | Any nonempty capture instructions unlock arbitrary folder creation | Gateway reproduction |

### SEC-01 — Vulnerable image decoder dependencies

Locations: `package.json:26`, `pnpm-lock.yaml:20`, `src/browser-capture.ts:85`, `src/worker.ts:126`, `src/worker.ts:170`.

The lockfile and installed package use `sharp 0.34.5`. The inspected host library reports libvips `8.17.3` and libheif `1.20.2`. `pnpm audit --json` reports two high-severity advisories affecting sharp:

- [GHSA-f88m-g3jw-g9cj, libvips vulnerabilities](https://github.com/lovell/sharp/security/advisories/GHSA-f88m-g3jw-g9cj): affected sharp versions below 0.35.0.
- [GHSA-rgj7-g3m4-5g8c, libheif vulnerabilities](https://github.com/lovell/sharp/security/advisories/GHSA-rgj7-g3m4-5g8c): affected versions below 0.35.4; upstream describes possible code execution under particular Linux conditions.

The browser receiver validates a supplied MIME string, not actual format. `inspectImage` passes bytes into sharp before the controller's later save-time magic-byte checks. The reproduction submits a harmless GIF labeled `image/png`, passes `validateBrowserCapture`, and successfully decodes it as GIF. A malicious page/server can likewise mislabel an image response. Public image research also feeds fetched bytes directly to sharp. CLI local-image imports do inspect magic bytes, but that does not protect these other paths.

Impact: the vulnerable native parsing surface is reachable from untrusted inputs. Successful exploitation could affect a worker and the data/capabilities available to it. Worker isolation limits impact; neither native code execution nor escape was attempted or demonstrated.

Fix: upgrade sharp to at least 0.35.4, resolving both reported advisories, and rebuild the worker image. Check the resulting native versions inside the image. Before any native parse, restrict inputs to supported formats, and disable unused sharp loaders as defense in depth. Pixel limits are useful but do not repair memory-safety defects. Add tests for GIF/AVIF/TIFF disguised as PNG/JPEG on browser upload and public-fetch paths, and rerun image and container tests.

### SEC-02 — Capture can read unrelated archive data and disclose it

Locations: `src/gateway.ts:511` (`/search`, `/read`), `src/gateway.ts:550` (`/fetch`), `src/gateway.ts:801` (CONNECT), `src/worker.ts:921` (capture tools).

Capture workers have archive search/read tools and outbound browsing/fetch tools in the same session. Search need not relate to the captured URL. A successful search authorizes full paginated reads. The resulting text can be placed in an arbitrary public URL and passed to `/fetch`, or sent through a public CONNECT tunnel. Public-IP filtering prevents many SSRF attacks but does not prevent disclosure to an attacker's public server.

The reproduction saves an unrelated synthetic private record, retrieves it through a capture gateway, and submits its source text in a collector URL. A fixture intercepts the fetch and confirms the complete synthetic value. This is a deterministic capability demonstration. Indirect exploitation requires persuading the model through hostile source content, or compromising its worker. A malicious capture-paired client can also submit explicit instructions trying to extract records into its capture result or a saved record.

`docs/validation.md` already acknowledges worker egress disclosure. This finding makes that accepted architectural risk concrete: unrelated archive content is exposed too. Instructions telling the model to ignore hostile text are not an authorization boundary. This matches the [OWASP excessive-agency risk](https://genai.owasp.org/llmrisk/llm062025-excessive-agency/).

Fix: separate public fetching from private archive access. A capture worker should normally receive only destination metadata and records for its exact source; perform broader organization in a separate session without public egress. Pre-fetch approved source material before exposing private context, or require explicit user approval for additional network requests through a controller-enforced policy. Domain allowlists alone are insufficient if attacker-controlled URL paths or query parameters can carry secrets to an otherwise allowed host. Test that capture cannot read unrelated record IDs and that private-data sessions cannot issue network requests.

### SEC-03 — Capture pairing permits persistent policy changes

Location: `src/receiver.ts:350`, especially POST `/capture-rules` at line 359.

`/archive` and `/commands` check for a management device, but policy writes only require any authenticated device. A capture device can fetch the current revision and overwrite root or folder `CAPTURE.md`. Revision checks prevent lost edits, not unauthorized edits.

Reproduction: a capture token gets HTTP 403 from `/archive`, then successfully reads and changes global capture rules with HTTP 200. This allows a stolen or lesser-trusted capture token to influence future captures across the vault and place persistent malicious guidance in the agent's context. It does not itself add new gateway endpoints or bypass their hard checks.

Fix: require management scope for policy writes and make the extension explain that requirement. If capture devices intentionally administer all capture policy, make that a separate explicit permission; the current two-scope design does not communicate the authority. Test every sensitive route against both scopes, including nested-folder policy writes.

### SEC-04 — Jobs have no device ownership checks

Locations: `src/receiver.ts:30` (job schema), `src/receiver.ts:414` (listing), subsequent GET/cancel/retry handlers.

Jobs contain no submitting-device ID. Every paired device can list the last 50 jobs, read any known job ID, and cancel or retry another device's work. Returned jobs include URL, title, result, error and potentially model-generated text and diagnostics. UUIDs are not a protection because the listing reveals them.

Reproduction: device A submits a synthetic capture. Independently paired capture device B retrieves A's result and cancels its failed job. Running and pending cancellation paths also have no ownership check; cancelling a running job aborts its processing.

Impact: cross-device privacy exposure and disruption of work. This is not cross-user tenancy, since the project has a single owner, but it expands the power of every capture token and undermines limiting damage from a lost browser/device.

Fix: persist submitting device ID; let capture devices list/read/retry/cancel their own jobs and management devices operate globally. Decide how legacy jobs are assigned. Return minimal progress summaries rather than complete worker diagnostics to capture clients. Add two-device tests for listing, direct reads, retry, cancel and duplicate capture IDs. If all capture clients are intentionally fully trusted peers, document this explicitly and do not call capture pairing a privacy boundary.

### SEC-05 — Management jobs can open outbound tunnels

Location: `src/gateway.ts:801`, condition at line 806.

The ordinary request handler denies `/fetch` to both `ask` and `manage`, but CONNECT checks only `intent === "ask"`. A management worker can therefore tunnel to arbitrary public port 443. CONNECT does not require a bearer token, although the private socket/inherited descriptor is itself a capability and is not publicly reachable.

Reproduction: create a management gateway and send CONNECT without Authorization. The handler returns HTTP 200 and attempts to connect to the supplied public IP. The test replaces the outbound connector with an in-memory duplex stream, so it creates no external connection.

Impact: compromised management workers can leak archive data despite the advertised lack of website access. The normal management model has no browsing or shell tool, so this is chiefly a worker-containment defect; it is not demonstrated model-only exfiltration or an unauthenticated public proxy.

Fix: apply a shared network-capability decision to both `/fetch` and CONNECT; deny both for `ask` and `manage`. Authentication on CONNECT can be added if Chromium's proxy setup supports it, but must not replace checking job intent. Test both transports for every intent.

### SEC-06 — Archive confidentiality depends on ambient umask

Locations: `src/vault.ts:110` (creation), `src/vault.ts:572` (artifact writes), `deploy/gg.service:13` (protective service umask).

Many archive directory/file writes omit explicit modes. With the common umask 022, a new vault is 0755 and original source files are 0644. The service supplies umask 0077, but direct CLI use does not. On a shared/traversable path, another OS account can read saved signed-in content and originals without pairing. A private parent directory prevents that access, so the finding is deployment-dependent, not evidence that this machine's real vault is exposed.

Fix: create private vault directories with mode 0700 and artifacts with mode 0600, or establish a restrictive CLI umask before creating data. Validate/report existing permissions without silently removing deliberate sharing. Include ownership and ACL inspection when deploying in a shared directory. Test direct CLI creation with umask 022. The reproduction demonstrates file modes, not a read by a second OS account.

### SEC-07 — Folder creation authorization tests only nonempty text

Location: `src/gateway.ts:326` (`/create-destination`).

The gateway allows destination creation whenever capture instructions contain any non-whitespace text. It does not bind the operation to an explicitly requested path. The reproduction supplies only “Summarize this page briefly.” and creates `UnrequestedFolder` successfully. A redirected model can create unwanted folders despite the documented requirement for an explicit request.

Impact is limited to new folders and associated history; this endpoint does not grant renaming, deletion or arbitrary filesystem writes. There is also no per-job count of created destinations.

Fix: use a structured controller-approved list of new paths, or require approval outside the model for a concrete path. Cap creations per job. Natural-language keyword checks would remain unreliable. Test that unrelated nonempty instructions cannot create a folder and that approved parents/children work.

## Additional hardening and unresolved coverage

These are observations or follow-up work, not additional demonstrated exploits:

- **Controller resource exhaustion:** worker cgroups do not bound all controller work. Gateway request bodies can reach 180 MB, `/fetch` retains text in `attributionEvidence` before checking the aggregate download budget, and over-budget fetch errors do not abort the job or preflight future fetches. IPC ignores write backpressure. Add concurrency limits, pre-reserved byte budgets, bounded evidence storage and cancellation on budget exhaustion. No deliberate OOM/stress attack was run.
- **Receiver abuse/retention:** pairing has high-entropy codes but no attempt/concurrency rate limit, and completed job metadata/history/originals have no overall quota or lifecycle policy. Pending-input limits do not bound total disk usage. Public exposure needs limits at both receiver and proxy. Retention/deletion must account for history, trash, failed inputs, job results and backups; deleting a visible record is not secure erasure.
- **Revocation:** revocation blocks future authentication but does not cancel already accepted jobs or necessarily stop a request authenticated before revocation. Define whether revocation should cancel that device's pending work; accepted-job durability is presently intentional.
- **Extension/media trust:** permissions for image hosts persist, and image retrieval includes credentials and follows redirects. Review private-network destinations, stale-tab/navigation behavior and cumulative host access. No cookie/token theft was demonstrated. Snapshot text excludes forms/editors and scripts; extension UI uses `textContent` rather than untrusted `innerHTML`.
- **Markdown output:** generated summaries and preserved `source.md` can contain arbitrary Markdown, including remote embeds and links. Safety depends on the consuming Markdown viewer. Disable automatic remote-resource loading where possible; treat exports as untrusted content. No viewer-specific XSS was demonstrated.
- **Filesystem races:** many paths use lstat-then-read, which is not race-proof against an attacker already able to mutate parent directories. Hard links are not generally rejected. Existing symlink protections meaningfully reduce ordinary traversal; do not treat a concurrently hostile writable vault as an isolation boundary. Exports and imports use `O_NOFOLLOW` on final files.
- **Supply chain/runtime:** the frozen lockfile and container source match are useful, but no OS/browser/container vulnerability scanner was installed. Npm audit does not cover the complete native image, kernel, Podman or Chromium. Pin/review image digests, rebuild after updates, and add recurring dependency and image scanning. The versioned image tag alone does not establish patch status.
- **Credentials:** controller state is outside the vault and JSON writes use 0600. `privateDirectory` does not tighten pre-existing permissions or check all parent components. Protect state backups and avoid placing real `.env.*`, keys or private replay snapshots under version control; `.gitignore` currently only names `.env` exactly. Pattern-only history scanning cannot rule out arbitrary-format secrets.
- **Live deployment:** actual public TLS/Host/Origin behavior, DNS/proxy configuration, firewall, service account, backup recovery, and installed host permissions were not audited against a deployed endpoint. No live provider, model red-team campaign or hostile native-image exploit was executed.

## Controls that held up

Rootless workers have no vault, provider credential, or runtime-socket mounts; a read-only image, limited writable scratch, dropped capabilities and resource limits are configured. Container tests exercised direct-network denial, host-file denial and Chromium sandbox startup. The existing worker image's 30 TypeScript source files match this checkout.

Receiver authentication uses random hashed device tokens, single-use expiring pairing codes, revocation, loopback binding, Host checks and restrictive browser Origin handling. Remote clients refuse insecure non-loopback HTTP and credential-bearing redirects. The public-fetch path validates resolved addresses, pins DNS results and repeats checks on redirects. No working private-network SSRF bypass was found in the reviewed tests/code.

Delete/merge confirmation lives outside the model; proposal fingerprints reject stale approvals. Paths/assets are constrained, exports stream known artifacts, originals are preserved, and history checks conflicts before undo. Model provider/model selection and usage budgets reside in the controller. These controls should be preserved while tightening permissions.

## Verification and reproduction

Run `node --import tsx .scratch/security-audit/reproduce.ts` from the repository. **Passing assertions mean the reported weaknesses still exist.** This is an audit probe, not a regression suite claiming the behavior is correct. Convert the corresponding assertions to denial/secure-mode expectations when implementing fixes. Temporary vaults are removed automatically.

- `pnpm test`: 64 passed.
- `pnpm typecheck`: passed.
- `pnpm test:worker`: 8 passed, 1 optional private replay skipped.
- `pnpm test:container`: 13 passed against the existing source-matching image.
- Chromium extension integration: 1 passed inside the worker image; host browser launch lacked an OS library.
- `pnpm audit --json`: 2 high-severity advisories, both affecting sharp; not two independent affected application packages.
- Seven harmless finding reproductions passed.
- Pattern scan: 1,043 reachable historical Git blobs, no matching private-key or recognizable provider-token patterns. This is not a dedicated secret-scanner guarantee and does not cover untracked files or external state.

Evidence is saved under [evidence](evidence/), including the dependency response, test logs, source comparison, redacted secret-pattern results and reproduction output. Extension verification is recorded separately in [extension validation](evidence/extension-validation.md).

## Remediation order

1. Upgrade sharp, disable unsupported decoders and rebuild/retest the worker.
2. Separate private archive access from public egress; fix management CONNECT at the same time.
3. Enforce capture/manage route permissions and device ownership of jobs.
4. Make archive permissions private by default and bind folder creation to explicit approved paths.
5. Add controller quotas/backpressure, retention rules and recurring dependency/image checks; validate the actual remote deployment before relying on public HTTPS.

After fixes, retain negative authorization tests at the receiver and gateway boundaries. Re-audit when adding tools, new input decoders, synchronization, additional users or new deployment targets. A clean test run today cannot guarantee safety against future dependency defects or changes in behavior.
