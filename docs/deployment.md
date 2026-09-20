# Deployment

[Home](../README.md) · [Installation](getting-started.md) · [CLI reference](cli.md)

Run one controller as the authoritative writer for your vault. Browser clients need only the extension; remote CLI clients need Node.js and the installed CLI. Clients do not need a container runtime.

## Prepare the controller

Follow [Getting started](getting-started.md) on a Linux host. An administrator may need to install Podman, crun and user-namespace helpers and configure subordinate UID/GID mappings. Confirm rootless operation with `podman info`, then run:

```sh
gg probe
pnpm test:container
```

Each worker is limited to 2 CPUs and 2 GiB RAM. Leave additional capacity for the controller, operating system and concurrent services. Account for originals, record history and worker images when sizing storage. GG has no production fallback to running the worker directly on the host.

The controller runs as an unprivileged user and holds provider credentials outside the vault. Keep unrelated services under separate accounts. Worker isolation is not a substitute for securing the controller host.

## Run as a service

From the installed checkout, under the controller user:

```sh
./scripts/install-service.sh
systemctl --user status gg
journalctl --user -u gg -n 100
```

The installer checks the worker image and isolation probe before starting the [user unit](../deploy/gg.service). It uses `~/.config/gg` and `~/.local/bin/gg`; adjust a copy for different locations. Node must be available on the unit's PATH. To keep the service running after logout, an administrator can enable lingering with `loginctl enable-linger USER`.

The unit sets a 3-GiB memory ceiling, a 512-task ceiling, a restrictive umask and cgroup delegation for rootless Podman. The installer does not change firewall settings. Schedule service updates between jobs; [update instructions](getting-started.md#update) include rebuilding the worker.

Use `gg pair` for capture access to that device’s jobs or `gg pair --scope manage` for global job administration, editing capture policy, management and archive downloads. Ownerless jobs from earlier versions remain visible only to management devices. Codes expire after ten minutes; device connections remain valid until revoked with `gg revoke DEVICE_ID`. `gg devices` lists paired devices. Startup logs may contain a short-lived pairing code.

## Remote connections

The receiver binds to IPv4 loopback by default. Choose SSH forwarding or an HTTPS reverse proxy. No particular private-network service is required.

### SSH forwarding

On the client, replace `USER@SERVER` with your SSH destination:

```sh
ssh -N -L 127.0.0.1:48123:127.0.0.1:48123 USER@SERVER
```

Use `http://127.0.0.1:48123` in the extension. Keep the tunnel open while submitting captures or checking status; accepted jobs continue on the server after disconnection.

For a remote CLI, obtain a management pairing code on the controller, then run on the client:

```sh
gg connect http://127.0.0.1:48123
gg list
```

### Public HTTPS

Configure DNS and a trusted HTTPS reverse proxy for your chosen hostname. Keep the GG receiver on loopback and set its exact public origin:

```sh
gg configure --public-origin https://gg.example.com
systemctl --user restart gg
```

The [Caddy example](../deploy/Caddyfile.example) is a starting template. Validate TLS, proxy body limits, Host/Origin handling, pairing, revocation and a capture from another machine. The extension requests access to the chosen HTTPS host during pairing. Do not expose the raw HTTP receiver port publicly.

## Archive copies and backups

Use [record downloads](files.md#browser-import-and-archive-downloads) for portable copies of selected records. GG does not implement general synchronization or automatic conflict merging.

For a consistent full-vault copy, stop the service and any separate CLI writers, then copy the vault with your preferred backup tool. For example:

```sh
# On the controller:
systemctl --user stop gg

# On the client; substitute the server account and vault path:
rsync -a USER@SERVER:/path/to/vault/ "$HOME/Gewolbe-copy/"

# On the controller after copying:
systemctl --user start gg
```

This example omits `--delete`; old moved/deleted records can remain in an existing copy. Copy into a fresh dated directory for an exact snapshot. The private controller state directory contains credentials and needs separate protection; do not distribute it with vault copies. Reconcile edits on the authoritative vault before refreshing another copy. Built-in encrypted, versioned backups remain a TODO.

See [known limits](validation.md) for crash recovery and validation boundaries.

GG creates new archive artifacts with mode 0600 and directories with mode 0700. Vaults with group/other root permissions produce a vault warning in job/probe results. Existing directories and files retain their permissions; review ownership, modes and ACLs on older/shared vaults and backups. Rebuild the worker whenever updating native image dependencies.
