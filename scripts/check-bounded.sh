#!/usr/bin/env bash
# Run verification in a disposable Linux user service. Never fall back to uncapped execution.
set -euo pipefail

if (( $# == 0 )); then
  echo 'Usage: scripts/check-bounded.sh COMMAND [ARGUMENT ...]' >&2
  exit 64
fi
for required in systemd-run flock; do
  if ! command -v "$required" >/dev/null; then
    echo "Bounded verification requires $required; no command was started." >&2
    exit 69
  fi
done
if [[ -z "${XDG_RUNTIME_DIR:-}" || ! -d "$XDG_RUNTIME_DIR" ]]; then
  echo 'Bounded verification requires a Linux user session (XDG_RUNTIME_DIR).' >&2
  exit 69
fi

# One shared lock prevents agents from stacking individually bounded jobs.
exec flock --nonblock --conflict-exit-code 75 \
  "$XDG_RUNTIME_DIR/gruenes-gewolbe-verification.lock" \
  systemd-run --user --wait --pipe --collect \
    --working-directory="$PWD" \
    --property=MemoryMax=2G \
    --property=MemorySwapMax=0 \
    --property=CPUQuota=100% \
    --property=TasksMax=256 \
    --property=RuntimeMaxSec=15min \
    --property=OOMPolicy=stop \
    /usr/bin/env "PATH=$PATH" CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 "$@"
