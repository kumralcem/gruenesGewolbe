# Perkele worker validation — 2026-09-20

Rootless Podman is now installed. The first run exposed image files copied from this checkout with owner-only permissions: root-owned `/app/package.json` and worker source were unreadable to the mapped worker UID. The Containerfile now explicitly makes the copied application files readable inside the image; host permissions and isolation flags remain unchanged.

- Reproduced `EACCES` with a direct mapped-UID file read and a new regression test before the fix.
- Rebuilt `localhost/gg-pi-prototype` on Perkele.
- All nine original container integration tests pass, including the browser/isolation probe, cancellation, retrieval and private browser snapshots.
- The new permission regression passes. Its initial 15-second timeout expired during the rebuilt image's first Podman startup (about 43 seconds); it now allows 90 seconds and passed on rerun. The full initial run therefore reported 9/10, followed by a passing focused regression run.
- `pnpm demo` passes: skip, queue, save, duplicate detection and retrieval through real Pi with fixture responses. Evidence: `.runs/demo-1789868459062/demo-results.json`.
- Typecheck passes. No provider sign-in, paid model calls, service deployment or host sudo changes were performed for this fix.

The earlier missing-Podman blocker below is resolved. Provider sign-in, live capture and service setup remain the next steps.

---

# Agent-led capture validation — 2026-09-16

Review base: `1aeb3714ec396c93192f3a30436302de4f5f0b23`. Implementation spec: `.scratch/agent-led-capture/PRD.md`.

- Typecheck passes. Unit suite: 36 tests pass, covering persistent budgets, retries, native model calls, pairing/revocation, remote CLI scopes, multi-record retries, manual edits, confirmation fingerprints and undo.
- `pnpm test:worker`: two real Pi tests pass private snapshot split capture, management delete preview/confirmation, preservation of multiple byte-identical images, and partial success with undecodable media, using deterministic provider stubs. This runs as a fixture on the host and makes **no isolation claim**.
- Chromium extension integration passes with optional instructions and automatic image collection from an authenticated fixture. Submitted bytes match the image, while cookie/form/script/hidden sentinels are excluded. Screenshot: `.runs/extension-evidence/capture.png`, visually inspected.
- Browser prerequisites were downloaded without privileged changes: Playwright Chromium, and Ubuntu browser libraries extracted under ignored `.runs/chromium-libs/root`. Reproduce here with `LD_LIBRARY_PATH="$PWD/.runs/chromium-libs/root/usr/lib/x86_64-linux-gnu" pnpm test:extension`. Normal prepared hosts do not need this workaround.
- `pnpm test:container`: all nine cases blocked by `spawn podman ENOENT`. `pnpm demo` is blocked by the same missing runtime. Previous-host container results below are historical, not evidence for this change on Perkele.
- Formatting, `git diff --check`, shell script syntax and `systemd-analyze --user verify deploy/gg.service` pass. Independent Standards/Spec review found seven issues, all fixed and rechecked; see [review record](../.scratch/agent-led-capture/review.md).
- Plain launcher installed at `/home/dev/.local/bin/gg`; `gg help` works. Service and optional Caddy template prepared; neither service nor public endpoint deployed. No domain is available. SSH-forwarding setup is documented.
- No live provider login/request, real website regression sweep, public TLS test, complete host-hardening audit, or existing-vault migration was performed. Whatbox backup remains deferred.

## Manual follow-up on the prepared host

Install the host prerequisites, build the worker, run its probe/container suite, sign in to the selected provider, and try a small private-page capture. Verify limits/allowance reporting, repeat capture, multi-record instructions, management preview/confirmation and undo. Connect a second device through SSH forwarding and verify revocation. Public HTTPS requires a later domain and proxy/ingress validation.

---

## Historical validation from the previous host and implementation

# Clean restart validation — 2026-09-11

The current change removes the Rust/Tauri application, promotes the Pi package to the root, adds Chrome capture and a local durable receiver, and writes Obsidian-compatible records. The fixed review base is `745f00dc2d986817c895fe83d8ec25b0d8fab4a3`, the completed prototype before this change.

## Automated evidence

- `pnpm typecheck` and `pnpm format:check`: pass.
- `pnpm test`: 23 tests pass. Includes snapshot field projection, receiver pairing/web-origin rejection/idempotency, interruption and retry across restart, malformed-job isolation, current retry status, exact selected-image enforcement, YAML edits, nested instruction headings, portable vault links with backslash titles, and the retained gateway/network/vault checks.
- `pnpm test:container`: 9 tests pass using real Pi and Chromium in rootless Podman, with deterministic model responses. Includes a private browser snapshot saved through Pi without re-fetching its source or selected image.
- `pnpm test:extension`: 1 real Chromium test passes. A fresh browser profile has an HTTP-only session cookie for an isolated local fixture. The extension captures its protected page and byte-identical protected image. Cookie, form, script, and CSS-hidden sentinels are absent from submitted content. Additional cases cover editable roots, form ancestors, and text selections spanning into a private form.
- Browser screenshot: `.runs/extension-evidence/capture.png`, generated by the extension test and visually inspected. It shows image selection, accepted capture status, and recent jobs. The local fixture uses a solid-color image; it is not a live artwork capture.

The browser test invokes the registered capture action handler in a real loaded extension. It does not automate an operating-system toolbar click. Browser-to-receiver and receiver-to-real-Pi paths are tested separately, avoiding live provider cost in the suite.

## Manual checks

1. Follow the [README](../README.md) to create a disposable vault, start `gg serve`, load the unpacked extension, and pair it.
2. Capture a signed-in page as an idea. Confirm its `record.md` summary and `source.md`; capture a chosen image separately and inspect the preserved file.
3. Close the capture tab after acceptance and reopen a capture tab to inspect recent results. Stop the receiver during a capture, restart/re-pair, and use Retry for the interrupted job.
4. Open the new vault in Obsidian. Follow `GG Index.md`, view an image embed, edit an idea's summary/add a property, then run `gg search` for the new wording.
5. For YouTube, open its transcript before clicking GG. Without a rendered usable transcript, expect a skip rather than audio transcription.

The no-key root demo also passed (skipped → queued → saved → existing, then retrieval). Its disposable Obsidian-ready vault is `.runs/demo-1789146565850`; open that folder in Obsidian and start at `GG Index.md`.

## Practical limits

Live authenticated X pages and live OpenRouter calls have not been validated in this change. Extraction varies by site; optional host access cannot bypass every image restriction. The worker/controller integrity limits from the [earlier prototype](prototype-results.md) remain. Hard crashes can leave lock or staging files; publication is atomic but does not promise fsync durability. The decision queue is listed and preserved, with no approval UI yet. No live archive was migrated, no extension was installed into the user's normal browser, and no hosted service was deployed.

## Review

The implementation workflow's separate Standards and Spec reviews are recorded in [review](../reports/restart-review.md) with all findings fixed and rechecked.
