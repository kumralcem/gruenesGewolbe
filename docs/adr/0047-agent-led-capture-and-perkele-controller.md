---
status: accepted
---

# Agent-led captures and the Perkele controller

The September 2026 interview supersedes the one-selected-image workflow in ADR-0042 and the local-only pairing/mode picker in ADR-0045. It revises ADR-0041's destination policy and ADR-0044's future-hosting status. ADR-0043's trusted-controller/isolated-worker boundary remains.

The extension is an interface to the same controller as the CLI. A capture requires only a page and optional instructions. The browser sends bounded sanitized structure, text, context and relevant image bytes. Public re-fetching is not required. Default scope is the article/post; optional instructions may include captured discussion. Default output is one coherent record with multiple images; explicit instructions may split it. Unknown classification goes to permanent Inbox. Missing content is visible and does not discard available content.

Recapture updates stable source/key matches, reuses prior instructions unless replaced, preserves human edits and appends a dated note. History supports individual and batch undo while detecting later edits. Partial successes persist. Ambiguous matches do not overwrite originals.

Management through `gg do`/`gg chat` may execute explicitly requested moves, edits and destination creation/renaming. Merge/delete require a concrete preview and a controller confirmation inaccessible to the model. Deletion retains recoverable trash. Capture agents cannot administer destinations.

Pi-native OAuth/API authentication belongs in controller state, outside the vault. Provider changes are manual; never silently fall back to paid usage. Durable request/token budgets cover all model work with one active job, bounded retries, warning/pause thresholds and limited explicit overrides. Provider-reported subscription allowance is distinct from local estimated usage; unavailable information is labeled unavailable.

Perkele is the primary host and authoritative vault writer. The receiver remains loopback-only; durable scoped device pairing supports remote CLI and browser clients through SSH forwarding or an optional HTTPS proxy. No domain exists yet, so public HTTPS remains unconfigured. Tailscale is optional. Podman isolates workers on the server, not on clients. Use existing transfer tools for vault copies; do not build a separate sync system into GG. Encrypted Whatbox backups are deferred TODO only.

Atomic file publication and a local history journal are prototype recovery mechanisms, not multi-file ACID transactions. Interrupted history requires explicit recovery. Live provider and host-isolation validation remains necessary before unattended use.
