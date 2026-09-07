Status: ready-for-agent
State: resolved
Assignee: root
Labels: wayfinder:task
Type: task
Mode: AFK
Parent: ../map.md
Blocked by: none

# Establish a safe measurement path

## Question

What resource bounds and observable signals will let future agents investigate test freezes and native UI delay without repeating an uncontrolled machine-wide freeze?

## Context

The user reports a freeze, not a shutdown; it occurred days before this investigation. The prior-boot journal query had no matching entries and establishes no cause. Playwright defaults are now one worker, but there is no memory cap and command-line overrides remain possible. Typecheck and browser discovery passed; no browser was launched.

## Work needed to unblock the decision

Inspect available resource limits, test entry points, and existing runtime instrumentation. Establish a bounded, cancellable measurement procedure and stop conditions before attempting any runtime reproduction. Prefer read-only evidence and small non-browser commands. Do not run build, browser, or stress suites together. Record which signals distinguish RAM pressure, CPU saturation, graphics hangs, or another cause without declaring any of them established.

## Resolution evidence

A documented runnable procedure with enforced bounds where available, commands actually checked, and explicit limitations. If reproducing the freeze remains unsafe, record that limitation instead of claiming a diagnosis. This task supplies the measurement prerequisite; it does not require fixing the entire test system.

## Comments

Resolution, 2026-09-06: added `scripts/check-bounded.sh` and `docs/bounded-verification.md`. The Linux user-service boundary enforces 2 GiB RAM, zero swap, one CPU quota, 256 tasks, and a 15-minute deadline. A session-wide flock prevents overlapping wrapped jobs. `pnpm test` routes through it; Playwright defaults to one worker. Missing bus access fails closed; the agent sandbox needed approved escalation to reach the user bus.

Validated a successful no-op and intentional exit 7 (correctly propagated). Six existing core capture tests passed through the runner in 501 ms service time, peak 123.4 MiB, zero swap. This proves a bounded measurement path, not a fix or diagnosis for the historical freeze. Native performance measurement and broader test validation remain outstanding.
