# Security remediation — 2026-09-20

Implements the seven findings from [the baseline audit](REPORT.md). The baseline report and its evidence describe commit `dce4125`; they are retained for traceability. The old `reproduce.ts` intentionally asserts the former vulnerabilities and is not a post-fix test. Current regression coverage lives in `test/security.test.ts`, `test/network-capability.test.ts`, and the updated worker integration suite.

| Finding | Implemented change |
|---|---|
| SEC-01 | Sharp upgraded to 0.35.4; browser uploads check actual supported magic bytes and MIME consistency; worker checks format before native decoding and blocks unused loaders. |
| SEC-02 | All capture intents lose broad archive search/read access. Snapshot/import jobs have no public egress. Public URL jobs receive only minimal source metadata; requesting private policy or saving irreversibly closes network access, cancels active downloads and destroys tunnels. Retained individual instructions are supplied only after that boundary. |
| SEC-03 | Capture-policy writes require management scope. Extension instructions explain how to reconnect with the appropriate pairing. |
| SEC-04 | Durable jobs carry submitting device IDs; capture devices see/control only their own jobs and receive minimal results. Management devices can administer all jobs. Device-scoped idempotency keys avoid cross-device import collisions. Legacy ownerless jobs are management-only. |
| SEC-05 | One network-capability decision governs fetch and CONNECT, including denial for both ask and management jobs. |
| SEC-06 | New vault files use 0600 and directories 0700 independently of umask, including restored directories. Opening/creating a vault reports permissive root modes through the existing vault-problem mechanism, without changing deliberate sharing. Existing files and ACLs require operator review. |
| SEC-07 | New folders require structured approved paths, limited to 16, supplied through the CLI `--create-destination` flag or the extension's folder fields. Free-text instructions and captured source cannot grant paths. Validation is shared across ingestion and gateway boundaries. |

Behavior changes are intentional: file imports cannot perform online artwork attribution; capture jobs cannot inspect unrelated archived notes for routing; folder creation needs explicit paths; capture-paired clients cannot edit shared policy or inspect other devices' jobs. Public URL jobs must finish source/media collection before reading private policy. Saved signed-in content still goes to the configured model provider.

Review: the implementation skill's standards and spec reviews identified missing asynchronous-network tests, duplicated approval validation, a cross-device import collision, and the lack of an existing-permission warning. These were addressed. Follow-up spec inspection found no further concrete defect. Transport tests cover existing tunnels, delayed DNS, both transports across every intent, and in-flight HTTP cancellation; image tests cover actual GIF, TIFF and AVIF inputs disguised as PNG.

## Verification

Final total: 90 passed, 1 optional skip. Rootless container tests: 13 passed. Unit tests: 67 passed. Pi worker tests: 9 passed, 1 optional private replay skipped. Chromium extension integration: 1 passed in the worker image. Typecheck and formatting passed. The dependency audit reports zero known vulnerabilities. The rebuilt image contains sharp 0.35.4, libvips 8.18.6 and libheif 1.23.2, and its source hashes match the checkout.

Final command results and evidence are recorded in the accompanying `evidence/fixed-*` files. The worker image is rebuilt from the changed source and dependency lockfile. The initial rebuild hit registry connection resets; rebuilding with host networking succeeded, after which the ordinary build reused the dependency layer. This changes build transport only; runtime worker network isolation remains enabled.

The baseline report's additional-hardening section remains a backlog, not a claim that every conceivable security concern is resolved. In particular, deployed TLS/firewall/ACLs and backups were not changed, the entire OS/browser image was not vulnerability-scanned, global archive/history retention limits remain future work, and no live-model prompt-injection campaign or native-code exploit was run. Revocation prevents new authenticated requests; already accepted jobs retain the existing durability semantics.
