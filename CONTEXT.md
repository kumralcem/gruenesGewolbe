# Grünes Gewölbe

A file-based personal archive. Pi handles capture, retrieval and explicitly requested management; a trusted controller owns the vault. See [README](README.md) and [architecture](docs/adr/0051-current-architecture.md) for current behavior.

## Domain language

- **Vault**: the portable directory containing records, preserved media, source copies and local history. No database is required.
- **Subvault**: an existing destination folder under `subvaults/`, including nested paths such as `Photography/Historic`. Discovered from disk, not limited to a registered flat list. Capture chooses one; explicit management may create or rename destinations. See [destination rules](docs/adr/0051-current-architecture.md#files-and-destinations).
- **Inbox**: permanent searchable destination when classification is uncertain.
- **Saved Item / Item Folder / Item Record**: a coherent thing worth keeping, its folder and editable `record.md`.
- **Capture**: interprets one source plus optional user instructions. Normally one record with relevant images; instructions may request several.
- **Capture Key**: stable identity of a record within a source, allowing individually matched updates after a split. Default `source`.
- **Browser Snapshot**: bounded visible text, sanitized structure, optional discussion/transcript and candidate image bytes from an explicitly captured tab; no cookie store or authenticated browser profile.
- **Source Copy**: preserved source text independent of generated summaries. **Summary** is a searchable interpretation, preserving actionable steps and conditions when relevant.
- **Primary File / Preserved Files / Thumbnail Preview**: main image, retained originals, and bounded display derivative. Previews do not replace originals.
- **Capture Date / Source Publication Date**: when GG saved an item / when the source was published if known. Neither implies ongoing accuracy.
- **Capture Job**: durable receiver input with pending/running/completed/partial/paused/failed/interrupted/cancelled state. Distinct from the legacy **Capture Queue** of unresolved decisions.
- **Operation / Batch / History**: one journaled change, a group of changes from one job, and retained file versions for conflict-aware undo.
- **Confirmation Proposal**: concrete delete/merge preview bound to record fingerprints, executed only by an explicit controller command.
- **Device Pairing**: a one-use short-lived code exchanged for a durable revocable capture or management token.
- **Application Budget / Provider Allowance**: GG's durable rolling request/token limits / subscription use reported by the provider when available. They are not interchangeable.
- **Vault Problem**: malformed or unsafe entry reported without hiding valid records elsewhere.
- **Tag / GG Index**: open-ended search label / generated Markdown links to records; neither replaces the records as source of truth.

## Boundaries

The controller holds provider credentials and the authoritative vault. Clients submit content and commands and can download record bundles. Workers have temporary storage and narrow controller capabilities; `ask` is read-only. Browser snapshots do not require public re-fetching. Remote clients can use SSH forwarding or an HTTPS reverse proxy. General-purpose sync, previous-application import and built-in encrypted backups are outside this implementation.
