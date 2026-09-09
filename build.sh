#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_DIR="$PROJECT_DIR/apps/desktop"

for tool in cargo cc pnpm pkg-config; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'Missing required tool: %s\n' "$tool" >&2
    exit 1
  fi
done

for library in gtk+-3.0 webkit2gtk-4.1; do
  if ! pkg-config --exists "$library"; then
    printf 'Missing required development library: %s\n' "$library" >&2
    exit 1
  fi
done

cd "$DESKTOP_DIR"
if [[ ! -x node_modules/.bin/tauri ]]; then
  pnpm install --frozen-lockfile
fi

pnpm tauri build --features tauri-runtime --no-bundle
printf 'Release binary: %s\n' "$PROJECT_DIR/target/release/gruenes-gewolbe"
