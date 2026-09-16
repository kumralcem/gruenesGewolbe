#!/bin/sh
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
target="${GG_BIN_DIR:-$HOME/.local/bin}"
mkdir -p "$target"
if [ -e "$target/gg" ] || [ -L "$target/gg" ]; then
  if [ "$(readlink -f "$target/gg")" != "$repo/bin/gg" ]; then
    echo "Refusing to replace an existing gg command at $target/gg" >&2
    exit 1
  fi
else
  ln -s "$repo/bin/gg" "$target/gg"
fi
"$target/gg" help
echo "Installed $target/gg. Add $target to PATH if needed."
