# Gruenes Gewoelbe

A local-first personal archive for saving, describing, and rediscovering material found while browsing.

## Current Implementation Slices

The first implemented slices establish the vault lifecycle, the first artwork saved item path, basic Paintings import, rebuildable metadata search, the first workbench operations, basic capture, cross-item organization, local-first duplicate candidate warnings, budgeted AI idea enrichment, and the first activity-log/agent CLI path:

- `gruenes-gewolbe-core`: archive core module for creating, opening, and validating a vault.
- `ggvault`: minimal CLI adapter for `create`, `open`, `validate`, `import-paintings`, `rebuild-index`, `search`, `capture-manual-text`, and `inspect-item`.
- `gruenes-gewolbe-desktop`: a Tauri 2 Linux desktop application with native folder dialogs, explicit active-vault handling, remembered vault roots, and a TypeScript/Vite workbench.
- Artwork saved items: local image files can be added to the Paintings subvault, preserved unchanged, recorded in Markdown, and reopened by stable item ID.
- Paintings import: supported image files can be copied from a local folder into the Paintings subvault through the core, CLI, and desktop shell.
- Metadata search: a derived index under `.gruenesgewolbe/` can be rebuilt from visible item records and queried through the core, CLI, and desktop shell.
- Workbench operations: the desktop shell can build a first-screen workbench snapshot with active vault, subvault navigation, collections, artwork grid items, Idea Sources, Review Queue, search results, and selected item details. It can also browse artwork grid items with rebuildable cached Thumbnail Previews, inspect item details, edit common Item Record fields, suggest safer folder renames after metadata cleanup, and accept, correct, or dismiss individual Review Reasons while deriving Review Status automatically.
- Capture: source links can be routed through a source-extraction boundary; successful extraction saves cleaned text, and blocked extraction returns a prefilled manual fallback prompt. Manual fallback text/image bundles preserve source-link provenance, source copies, copied image bytes, saving reason, and review status.
- Organization: saved items can use a vault-level tag registry with aliases, durable collection files with item backreferences, and item links without changing their home subvault.
- Duplicate candidates: import and capture workflows record nonblocking duplicate warnings from local file fingerprints, source links, import provenance, and strong descriptive metadata.
- AI enrichment: captured ideas can be enriched through a provider boundary with visible budget modes, minimal cleaned-text context, accepted summaries/tags, high-confidence metadata acceptance for unknown placeholder fields with provenance, staged metadata suggestions for unsafe or conflicting changes, additive better-file candidates that do not replace the primary file, image-only artwork metadata suggestion calls, and hidden AI cost log hooks. OpenAI provider config is stored in user-specific app state rather than the vault. Normal tests use fake providers; no live OpenAI call is required.
- Activity log and agent CLI: capture, import, metadata rebuild, enrichment, AI cost, and selected error events append to a hidden noncanonical activity log. The CLI exposes structured TSV-style output for validation, opening, import, rebuild, search, capture, item inspection, and selected errors.

The current test seam is the archive core public interface, with thin CLI, desktop command, and browser workflow tests around the same behavior.

## Linux Desktop

The first desktop milestone targets Linux and produces a local binary rather than an installer. AppImage and other packaging formats are intentionally deferred until the application workflow is stable.

Required tools are Rust, Node.js/pnpm, a C compiler, `pkg-config`, GTK 3 development files, and WebKitGTK 4.1 development files. On Fedora, the relevant GUI packages include `gtk3-devel` and `webkit2gtk4.1-devel`.

Launch the development application:

```sh
./run.sh
```

Build the release binary without creating an installer:

```sh
./build.sh
./target/release/gruenes-gewolbe
```

## Verification

Run the Rust test suite with:

```sh
cargo test
```

Run the desktop type checks and browser workflow tests with:

```sh
cd apps/desktop
pnpm install --frozen-lockfile
pnpm exec playwright install chromium
pnpm typecheck
pnpm test
```

The browser tests use controlled desktop-command responses; the Rust command tests cover persistence and the on-disk Vault behavior.

On systems without a C compiler on `PATH`, the current suite can be run through Rust's musl target and bundled linker:

```sh
RUSTFLAGS='-C linker=/home/cemkumral/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld -C link-self-contained=yes' \
  /home/cemkumral/.cargo/bin/cargo test --target x86_64-unknown-linux-musl
```
