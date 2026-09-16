#!/bin/sh
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
command -v podman >/dev/null || { echo "Install rootless Podman and uidmap first; see docs/deployment.md." >&2; exit 1; }
[ -x "$HOME/.local/bin/gg" ] || { echo "Run scripts/install-cli.sh first." >&2; exit 1; }
[ -f "$HOME/.config/gg/settings.json" ] || { echo "Run gg init and gg configure first." >&2; exit 1; }
podman image exists localhost/gg-pi-prototype || { echo "Build the worker image first." >&2; exit 1; }
"$HOME/.local/bin/gg" probe >/dev/null
mkdir -p "$HOME/.config/systemd/user"
install -m 600 "$repo/deploy/gg.service" "$HOME/.config/systemd/user/gg.service"
systemctl --user daemon-reload
systemctl --user enable --now gg.service
systemctl --user status gg.service --no-pager
