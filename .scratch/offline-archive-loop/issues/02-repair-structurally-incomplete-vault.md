Status: ready-for-agent

# Repair a Structurally Incomplete Vault

## Parent

.scratch/offline-archive-loop/PRD.md

## What to build

Build a safe recovery path for a recognizable format-version-2 Vault whose required structural directories are missing. Opening should present a repair proposal, allow the user to recreate only safe empty structure, and then continue into the workbench without rewriting malformed or existing canonical content.

## Acceptance criteria

- [ ] Opening a Vault with a valid configuration but missing required structural directories produces a repair proposal instead of silently changing the Vault or reporting it as wholly unusable.
- [ ] The repair screen lists each directory that would be created and requires explicit confirmation.
- [ ] Confirmed repair creates only missing safe empty directories and then opens the repaired Vault.
- [ ] Repair never overwrites existing files or directories and never rewrites malformed Item Records.
- [ ] Invalid configuration and unsafe structural conflicts remain clear errors rather than being treated as repairable automatically.
- [ ] Cancelling repair leaves the selected folder unchanged.
- [ ] Archive-core, Tauri-command, and browser-level tests cover repairable, cancelled, non-repairable, and successful-open outcomes.

## Blocked by

- .scratch/offline-archive-loop/issues/01-launch-and-reopen-format-v2-vault.md

