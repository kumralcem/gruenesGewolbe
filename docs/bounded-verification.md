# Bounded local verification

The user has experienced whole-machine freezes during tests. The cause is unconfirmed. Browser concurrency alone does not bound memory, so run verification through the Linux user-service wrapper:

```sh
bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-core --test capture -- --test-threads=1
```

From `apps/desktop`, `pnpm test -- tests/browser/url-capture.spec.ts` uses the same wrapper. Playwright defaults to one worker; do not override it upward on this machine. `pnpm typecheck` and `pnpm exec playwright test --list` do not launch browsers.

The wrapper enforces 2 GiB RAM, no swap, one CPU worth of execution, 256 tasks, and a 15-minute deadline. One session-wide lock prevents overlapping wrapped jobs. Rust compilation and tests default to one job/thread. A failing, timed-out, or memory-killed check is a failed check, not proof of a product regression or permission to rerun it uncapped. Systemd reports the job's peak memory and outcome.

The runner requires a Linux systemd user session and access to its bus. In an agent sandbox, an approved tool escalation may be needed to reach that bus; the wrapper never silently falls back to unrestricted execution. Exit 75 means another verification job holds the lock. User-service jobs do not automatically inherit arbitrary shell environment variables; normal verification should not need provider credentials.

These limits constrain the verification job, not the rest of the desktop. They do not prove the earlier freeze was caused by memory exhaustion. Start with a focused file, inspect the result, and expand only when justified. Avoid running the native app or separate builds alongside the check. Live provider calls and native interaction remain distinct from mocked browser coverage.
