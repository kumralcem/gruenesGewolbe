---
status: accepted
---

# Private Hosted Interface Over a File Vault

Build the CLI first, with capture, search, and ask operations reusable by a later private browser application for the user. The hosted application runs the GG controller and sandboxed workers on a server and uses one authoritative writable server vault, with a backup on the user's computer. This preserves the ordinary-file archive while enabling remote browser use without introducing independent local/server editing and synchronization conflicts.

The hosted vault remains a directory of saved content and Markdown records on persistent storage, independently backed up and outside disposable worker containers. Model credentials remain with the controller. Access to GG is private. Source-session capture was subsequently authorized through a local browser extension in [ADR-0045](0045-browser-session-capture.md). The hosted CLI and GUI route mutations through the same server controller instead of independently writing the backup.

Hosted capture jobs continue after browser disconnection, with file-based status and outcomes available on return. Processing state is distinct from the Capture Queue for user decisions. Foreground execution remains sufficient for the initial local CLI. Hosting adds the web interface, private access, job lifecycle, and deployment/backup management rather than changing the capture rules or requiring a database.

Remote browser interaction requires a running service, while the preserved archive remains inspectable independently as ordinary files. Multi-user accounts, a public gallery, and two-way local/server synchronization are outside this design. The hosting provider, access mechanism, backup procedure, and deployment timing remain implementation decisions; this design agreement is not an instruction to publish or migrate the existing vault now.
