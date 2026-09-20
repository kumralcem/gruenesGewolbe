# GG on Perkele

Perkele holds the working vault and model credentials. The extension submits browser snapshots; clients need neither a container runtime nor Tailscale. No domain is configured. Start with SSH forwarding, using the same address in the extension and remote CLI.

## Host preparation

Perkele's 4 vCPUs / 8 GB RAM / 80 GB disk are suitable for one bounded worker and the expected sub-10-GB vault. History, originals and worker images also consume disk. UFW, unattended-upgrades and Tailscale were active, but effective firewall and SSH settings could not be inspected with available privileges. This is not a completed hardening audit.

Rootless Podman, crun and user-namespace helpers are now installed on Perkele; the worker isolation checks passed on 2026-09-20. On a fresh host, an administrator must install these distribution packages and validate subordinate UID/GID mappings and rootless operation. GG deliberately has no production fallback to running the worker on the host. The controller runs as an unprivileged user. Keep future website processes under a separate Unix account or move them to another host; a worker container does not isolate the trusted controller from other processes running as its user.

```sh
# From the installed checkout, under the controller user:
pnpm install --frozen-lockfile
./scripts/install-cli.sh
pnpm build:worker
gg init --vault "$HOME/Gewolbe"
gg login openai-codex
gg configure --provider openai-codex --model gpt-5.6-luna
gg probe
pnpm test:container
pnpm demo
```

Use an empty new vault for initial validation. Do not migrate the existing archive implicitly. API alternatives are documented in the README. Sign-in belongs on this host; provider credentials must not be copied to browser clients or vault mirrors.

## Run as a service

After the commands above pass:

```sh
./scripts/install-service.sh
systemctl --user status gg
journalctl --user -u gg -n 100
```

The installer checks the worker image and isolation probe before starting the [user unit](../deploy/gg.service). It uses `~/.config/gg` and `~/.local/bin/gg`; adjust a copy for a different location. Node must be available on the unit's PATH. Ensure the controller user has lingering enabled if it should survive logout (`loginctl enable-linger USER`, administered on the host). The service has a 3-GiB memory ceiling, 512-task ceiling, restrictive umask and cgroup delegation for rootless Podman. No host firewall changes are made by the installer.

`gg pair` issues a short-lived capture code; `gg pair --scope manage` issues a management code. Avoid sharing the receiver's log because startup prints a short-lived code. `gg devices` and `gg revoke ID` work on the controller host. Device tokens are hashed server-side and individually revocable.

## Connect without buying a domain

On the laptop or desktop, with your existing SSH configuration:

```sh
ssh -N -L 127.0.0.1:48123:127.0.0.1:48123 perkele
```

Use `http://127.0.0.1:48123` in the extension, then paste a code from `gg pair` on Perkele. The network hop is protected by SSH. Keep the forwarding session open while submitting captures or checking status; accepted jobs continue when disconnected. SSH may use the public address or your existing Tailscale route. Tailscale is optional.

For the CLI, install the checkout/dependencies/launcher on the client (no worker image needed), issue `gg pair --scope manage` on Perkele, then on the client:

```sh
gg connect http://127.0.0.1:48123
gg list
gg do 'Move the note about architecture into Inbox'
```

Do not expose port 48123 directly. The receiver binds only IPv4 loopback, checks Host/Origin and authenticates every request.

## Optional public HTTPS later

Once you choose a domain, point its DNS at Perkele, install a trusted HTTPS reverse proxy and allow the required HTTPS/certificate-validation ingress. Configure GG's exact public origin and restart the service:

```sh
gg configure --public-origin https://gg.example.com
systemctl --user restart gg
```

[deploy/Caddyfile.example](../deploy/Caddyfile.example) is a template, not installed configuration. It uses Caddy's [reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) and [request body limit](https://caddyserver.com/docs/caddyfile/directives/request_body). The proxy forwards to loopback and preserves the original Host. Validate proxy/TLS, firewall, pairing, revocation and a capture from another machine before considering public deployment complete. A browser can request access to the chosen HTTPS host during pairing. No Tailscale, client certificate or local daemon is required for that path.

## Laptop and desktop vault copies

Keep Perkele as the authoritative writer. Use existing file-transfer tools for one-way mirrors; GG does not implement general-purpose synchronization or automatic conflict merging. A consistent copy can be made while the service is stopped and no separate CLI writer is running:

```sh
# On Perkele:
systemctl --user stop gg

# On a client; replace USER and its home path:
rsync -a perkele:/home/USER/Gewolbe/ "$HOME/Gewolbe/"

# On Perkele after copying:
systemctl --user start gg
```

The example intentionally omits `--delete` so a mistaken deletion is not propagated automatically. It may leave old moved/deleted records in the destination; for an exact snapshot, copy to a fresh dated folder. Do not sync `~/.config/gg`, which contains provider/device credentials. Local notes may be edited, but edits must be explicitly reconciled on the authoritative vault before replacing a mirror. Perform routine archive edits through `gg do`/`gg chat` from any connected machine.

## Deferred

- TODO: encrypted, versioned Whatbox backups; no backup service, key or job installed now.
- Public domain/TLS setup, pending an actual hostname.
- Complete host hardening audit; worker isolation tests have passed, while live transport reliability remains an open issue (see [validation](validation.md)).
