# Agent-led capture — design interview

Status: interview closed; implementation tracked in PRD.md. Later hosting decisions below supersede earlier discussion.

## Agreed decisions

- The extension is a browser interface to the shared GG CLI/backend. Open it by toolbar or shortcut, optionally enter instructions, and submit. No required capture type, destination, or image selection.
- Browser captures supply relevant rendered text, sanitized structure, and image content from the user's browser session. Processing must not depend on publicly re-fetching the source or exporting browser credentials.
- The agent interprets the supplied content and chooses an existing subvault. Optional external research is an enhancement, not a prerequisite for saving.
- By default, one link produces one coherent record with multiple relevant images when appropriate. Explicit instructions may request multiple records.
- For multiple records, save successful records immediately and retry only unfinished work.
- Save captures with uncertain classification to a permanent, searchable Inbox.
- Repeat captures update the previous capture rather than creating another record by default. Include a note documenting the update. Use a dated note and stable keys for clearly matched split records.
- Use Pi's own Codex subscription integration, keeping credentials and refresh in the trusted controller.
- On explicit user instructions, the management agent may move/edit records and create/rename subvaults directly. Merges and deletions require a preview and confirmation; deletion uses recoverable trash. Automatic capture continues to use existing subvaults or Inbox.
- Repeat-capture updates preserve user notes and manually edited fields, retain the user's version on conflict and flag the proposed change, and append a dated change note. Keep local version history and retained media with restoration support.
- Save available content when some media cannot be downloaded, clearly mark missing media, and support retry without claiming complete success.
- Default capture scope is the linked post/article, its images, captions, and attribution. Exclude navigation, recommendations, and replies by default; user instructions can include a thread/discussion.
- Once the snapshot is received, work continues after the extension or source tab closes. Recent captures show progress, outcomes, missing content, and decisions needed.
- Provider switching, including subscription-to-paid-API switching, is manual. Alert the user when subscription allowance is nearing exhaustion, subject to available provider usage information.
- The user wants the program to live on a remote machine; discuss deployment after the design interview. Do not assume a local background service is the final deployment.
- Add protection against aggregate usage exhaustion from bugs. Per-job limits exist; global limits and their behavior are being discussed.

## Agreed usage protection

- Controller-wide limits cover captures, retries, retrieval, and management chats, with durable counters surviving restarts.
- Configurable rolling hourly and weekly request/token budgets, initially one active model job. Starting request ceilings: 50 calls/hour and 300/week; token-budget values remain to be calibrated. Bound input size per request too. Application budgets do not represent subscription percentages.
- Warn at 80% of an application budget; block new model calls at 100%. Continue accepting snapshots into the bounded queue, explain pauses and reset times, and allow explicit limited overrides. Never switch providers automatically.
- Use provider-reported subscription allowance for near-exhaustion warnings when available; clearly report unavailable allowance rather than deriving a subscription percentage from local token/request counts.
- At most two delayed automatic retries for transient failures; all attempts count against budgets. Pause the provider after repeated consecutive failures; authentication and exhausted-allowance errors pause immediately. Exact consecutive-failure threshold remains an implementation setting.
- Show usage warnings in the extension and CLI, with optional browser notifications. Include active provider, application budgets, and provider-reported allowance when available.

## Additional requested capabilities

- Natural-language commands to an agent that can manage the vault, beyond the existing read-only `ask` command. Management authority is agreed above; the interface details remain to be resolved.
- Expose a plain `gg` executable without requiring `pnpm gg` for daily use. Prefer a lightweight local launcher/install over burdensome packaging; implementation is pending.

## Topics resolved by closing defaults

- Update history, preservation of user edits, and identity when a source produced multiple records.
- Capture size limits, missing media, and handling genuinely ambiguous source scope.

## Closing defaults and deployment discussion

- The user prefers ordinary implementation defaults over further edge-case product choices. Proposed defaults at interview close: clearly matched split records update individually; uncertain matches remain untouched and flagged; prior source instructions carry forward unless replaced.
- Undo also covers management actions and batches, protecting subsequent edits against overwrite.
- Available machines: a new development VPS intended for public websites, an existing seedbox of uncertain execution capability, a laptop, and a desktop.
- User does not want personal vault storage on the public development VPS, is uncomfortable with plaintext personal files on the seedbox, and wants capture access plus vault copies across machines.
- Previous Syncthing arrangement felt unreliable; user wants to move away from it. Do not build a general-purpose sync/backup system into GG.
- Additional recurring hosting costs are strongly disfavored. Seedbox runtime capability, provider backup guarantees, and desired offline/edit synchronization semantics are unverified.

## Revised hosting direction

- Prefer Perkele as GG's primary host, holding the working vault and model credentials. The user already trusts this machine with credentials; a separate future website host is an option if needed.
- TODO (deferred): encrypted versioned backups to Whatbox, without model credentials there. Do not include backup setup in the initial deployment. Client machines retain vault copies; general-purpose sync/backup tooling remains outside GG implementation scope.
- Perkele is a CX33: 4 vCPUs, 8 GB RAM, 80 GB nominal storage. The user's vault is small and expected to stay under 10 GB in the near term.
- Read-only assessment found approximately 63 GiB free disk space and 4.5 GiB available RAM. One worker at a time is the proposed initial concurrency.
- UFW, Tailscale, and unattended-upgrades services are active. Firewall rules and effective SSH configuration could not be inspected with available privileges; no complete hardening assessment has been made. Development services listen on all IPv4 interfaces, so actual access rules need verification before deployment.
- Podman was not found on the current user's PATH. Worker runtime installation and isolation validation remain necessary on Perkele.
- Tailscale is already present between the user's machines, excluding Whatbox, but must not be a mandatory GG dependency. Standard authenticated HTTPS is the intended remote interface; an existing private network may optionally carry it. Minimize client setup to extension installation and server pairing; do not require a client-side container runtime or additional networking software.

## Endpoint decision

- No domain is available. Leave public HTTPS unconfigured, keep the endpoint configurable, and document SSH forwarding using the existing connection to Perkele.
