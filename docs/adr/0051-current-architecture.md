---
status: accepted
---

# Current architecture

Consolidates the still-active decisions from ADRs 0040–0050. Their historical text and superseded workflows remain in Git history. This is a documentation consolidation, not a runtime change. Commands and setup belong in [README](../../README.md); operational limitations belong in [validation](../validation.md).

## Controller and workers

Pi's supported SDK supplies the agent runtime without a fork. The trusted controller owns credentials, budgets and the file vault. Each agent job starts a fresh rootless Podman/crun worker with temporary scratch, a read-only application image, no direct network and no host/vault/credential/runtime-socket mounts. A private inherited descriptor connects narrow capabilities and constrained public-network access to the controller. No privileged or nested container runtime is required.

The same controller serves CLI commands and paired browser clients. `ask` exposes read-only archive search/read; general management requires explicit user instructions, with concrete delete/merge proposals confirmed outside the model. Capture may create only exact paths approved through the structured createDestinations field (CLI flag or extension field), at most 16 per job. Free-text instructions cannot expand that list. Source content never authorizes management or policy changes.

## Capture and preservation

An explicit extension gesture captures bounded visible text, sanitized structure, available transcript and relevant image bytes from the user's signed-in tab. No mandatory mode or image picker. Browser cookies, credential stores, form contents and authenticated profiles stay outside the snapshot. Private captured content can be sent to the configured model provider; browser snapshots and local imports have no public-network capability. Public URL captures collect public material first; reading private capture policy or saving permanently closes public egress and existing tunnels. Capture workers cannot search/read unrelated archive records, and public-phase source context excludes existing summaries and saved instructions. Direct CLI URL capture remains public. No audio downloading/transcription.

Default output is one coherent record with relevant images; explicit instructions may request multiple records with stable capture keys. Unknown classification goes to Inbox. Text-only idea captures may omit irrelevant images without reporting their absence as partial; genuine missing content remains visible. Full source text is preserved independently of generated summaries within documented bounds.

Recapture updates source/key matches, preserves earlier originals and human edits, retains existing placement and prior instructions unless replaced, and records conflicts. Successful parts of a split persist across retry; ambiguous matches do not overwrite records. Moving records remains explicit management work. Legacy single-image commands retain conservative same-image upgrade behavior and may keep visually similar but differently encoded duplicates.

Local image imports use exact content hashes as provenance. The controller requires matching supplied image bytes and preserves the original. Artwork title/artist/year attribution distinguishes filename hints, uncertainty and source-supported claims. Fetched excerpts are evidence, not proof of correct artwork matching or source authority.

## Files and destinations

The vault is portable ordinary files: `subvaults/<path>/items/<id>/record.md`, preserved media, source text and internal asset/baseline/history files. Markdown with YAML properties and relative media links works in Obsidian without a plugin or database. Search reads records; `GG Index.md` is derived. User properties, notes and edited summaries survive managed updates.

Discover destination directories recursively before operations, excluding hidden folders, `items` trees and symlinks. Existing nested folders guide routing; empty folders need no registration. Parent destinations may contain records and children. Explicit folder renames update descendants. Directory-aware history preserves empty folders; older file-only folder undo is rejected when ownership cannot be established. Arbitrary external rearrangement and asset-link repair are not implemented.

Atomic publication and journaled undo are prototype recovery mechanisms, not multi-file transactions or fsync-level guarantees. Later edits block unsafe undo. Hard crashes can leave locks or staging entries requiring inspection.

## Policy and budgets

Root and destination-folder `CAPTURE.md` files supply inherited capture defaults. Each file is bounded to 16 KB (64 KB combined), and cannot expand tool permissions. Individual capture instructions take precedence. Existing files are preserved; settings edits use revision checks, the vault lock and history. External editors cannot participate in that lock. Changes affect future jobs/explicit recaptures, not automatic rewrites. The agent reads the chosen destination’s root-to-leaf policy chain before saving. Nonempty folder overrides require that read. Resolved chains retain revisions and text across retries, without freezing a split plan merely by reading guidance. Settings show individual and effective instructions.

All model work using a controller state directory shares a job lock, rolling budgets and bounded retries. Provider switching is manual. GG usage counters and provider-reported subscription allowance are separate. Explicit window resets retain request history; temporary and hash-scoped collection grants have finite request/token/expiry limits and never bypass provider-error pauses or per-job limits.

## Hosting and clients

One controller is the authoritative writer. Provider authentication belongs in private controller state, outside the vault. The receiver binds loopback and uses durable, revocable capture/manage pairing tokens. SSH forwarding works without a domain; public HTTPS is optional. Capture policy edits require management scope. Capture devices can access only their own durable jobs; management devices can access all jobs, including legacy ownerless jobs. Capture responses omit worker diagnostics. Accepted browser jobs survive disconnects; pending work resumes, interrupted work requires explicit retry. Per-job state differs from the legacy unresolved-decision queue.

File mirrors are not independent writable replicas. General sync/conflict merging, migration from the removed application, built-in encrypted backups are not implemented. Management-paired clients can browse records and download streamed portable bundles or originals-only bundles. The extension also uploads local images through the existing durable queue. Browser capture remains passive; future extraction options remain in the handoff.
