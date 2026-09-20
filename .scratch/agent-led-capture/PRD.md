# Agent-led capture and hosted GG

Status: ready-for-human — implementation complete; live provider and service setup remain

## Outcome

A user opens the browser extension, optionally provides instructions, and sends a self-contained browser snapshot to one GG backend on Perkele. Pi chooses content and destination, saves one coherent record or explicitly requested multiple records, and supports natural-language vault management. No capture mode/image selection, client worker runtime, or Tailscale requirement.

## Acceptance criteria

1. Versioned snapshots include sanitized page structure, text, captions and bounded multiple image bytes; missing media is reported and available content remains savable. Scope defaults to the primary article/post; instructions may include discussion. The browser snapshot is authoritative and never requires public re-fetching.
2. Multiple records and images; uncertain classification goes to permanent Inbox. Partial success survives failures/retries. Recaptures update matched records, reuse prior instructions, preserve manual edits, record dated changes, and retain restorable history. Uncertain matching is flagged without overwriting.
3. `gg do` and `gg chat` manage records and subvaults. Explicit moves/edits/create/rename execute directly. Delete/merge require concrete previews and controller-issued confirmation; deletion is recoverable. Undo supports individual operations and batches and rejects conflicts with subsequent edits.
4. Pi-native Codex OAuth plus existing OpenAI/OpenRouter API keys; all real credentials remain in the controller. Manual provider switching. Report subscription allowance when available, otherwise explicitly unknown.
5. Durable controller-wide rolling hourly/weekly request and token budgets, per-request input bounds, at most one model job; defaults 50 calls/hour and 300/week. Warn at 80%, pause at cap, bounded explicit overrides; no paid fallback. Up to two transient retries, budgeted attempts, circuit-breaker pause, immediate pause for auth/quota failures. Paused captures remain queued; extension and CLI show usage and optional browser alerts.
6. Plain `gg` launcher; local and paired remote CLI share backend operations. Durable revocable per-device credentials, authenticated HTTPS support with optional Tailscale, loopback backend behind trusted reverse proxy. Bounded upload/pending storage, durable acceptance, restart/retry/status/cancel semantics.
7. Perkele deployment files and operating instructions, isolated worker, one active job, persistent configuration outside vault, client-readable vault mirrors using existing transfer tools. Deployment requires host prerequisites and HTTPS domain; don't silently disable isolation. Encrypted Whatbox backups are TODO only.

## Validation

Use boundary tests for capture validation, version history/manual edits, management confirmation and undo, persistent budgets/retries, device revocation and remote requests. Run typecheck and focused tests throughout; full unit suite, container and browser integration, fixture demo, then Standards and Spec review. Record environmental blockers honestly; do not claim live OAuth/model/deployment validation without doing it.

## Work sequence

- [x] New capture contract and multi-image extension
- [x] Record updates/history, multi-record saves, Inbox, management and undo
- [x] Controller model provider/OAuth and durable usage guard
- [x] Worker tools and prompt integration
- [x] Paired receiver/remote CLI and launcher
- [x] Deployment/docs and full validation
- [x] Independent review, fixes, commit

## Operational follow-up (environmental blockers)

- [x] Administrator installs rootless Podman/crun/uidmap on Perkele; build worker and pass isolation probe/container suite/demo (validated 2026-09-20).
- [ ] User signs into selected provider; verify a live capture and allowance reporting.
- [ ] Enable the prepared service and pair client devices through SSH forwarding.
- Public HTTPS intentionally remains unconfigured: no domain available. Whatbox encrypted backups remain deferred TODO.

See `docs/validation.md` for evidence and `review.md` for independent review findings and fixes.
