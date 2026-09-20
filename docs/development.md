# Development

[Home](../README.md) · [Architecture](adr/0051-current-architecture.md) · [Domain vocabulary](../CONTEXT.md)

## Validation

```sh
pnpm typecheck
pnpm format:check
pnpm test
pnpm test:worker                     # real Pi + stubbed provider; no isolation claim
pnpm test:container                  # actual Podman boundary
pnpm test:extension                  # Chromium + cookie-protected fixture
pnpm demo                           # deterministic container demo, no provider key
```

See [validation](validation.md) for test evidence and current live reliability blockers. Live provider sign-in, arbitrary website behavior and public deployment require separate checks; fixture success does not establish them.

## Visual assets

Editable logos and icon-generation instructions live in [assets/brand](../assets/brand/README.md). Committed extension icons require no client-side build.
