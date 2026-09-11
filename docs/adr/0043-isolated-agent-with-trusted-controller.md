---
status: accepted
---

# Isolated Agent With a Trusted GG Controller

Run Pi and its browsing, command-line, and agent-written script tools in an unprivileged container with a temporary working directory. A trusted GG controller outside that container holds provider credentials and controls archive access. This gives the agent flexibility to handle varied sources while enforcing the user's restrictions on credentials, vault administration, and saved files through code rather than prompting alone.

Model requests pass through a constrained gateway which adds the real provider key outside the worker and enforces job limits. Archive operations expose permitted searches, reads, new captures, queued decisions, and same-image upgrades; the worker receives no unrestricted host filesystem access. Saves validate the existing destination and preserve earlier image versions. `gg ask` has no archive-write capability. Ordinary valid captures still save automatically.

Use Pi's supported SDK and customization points as the initial integration path. A fork requires a demonstrated limitation. Container nesting is unnecessary: the controller may run as a host process or as a separately configured sibling container. Do not give the worker a container-runtime socket, privileged execution, authenticated browser profile, or mounts of real credentials or the live vault. The implementation uses rootless Podman/crun, an inherited private descriptor, constrained public-network proxying, and per-job resource bounds. Automated isolation checks and their practical limits are documented in the README and validation report. Browser snapshots do not change this boundary; see [ADR-0045](0045-browser-session-capture.md).
