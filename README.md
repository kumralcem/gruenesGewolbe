# Gruenes Gewoelbe

A local-first personal archive for saving, describing, and rediscovering material found while browsing.

## Current Implementation Slices

The first implemented slices establish the vault lifecycle, the first artwork saved item path, basic Paintings import, rebuildable metadata search, the first workbench operations, basic capture, cross-item organization, local-first duplicate candidate warnings, budgeted AI idea enrichment, and the first activity-log/agent CLI path:

- `gruenes-gewolbe-core`: archive core module for creating, opening, and validating a vault.
- `ggvault`: minimal CLI adapter for `create`, `open`, `validate`, `import-paintings`, `rebuild-index`, `search`, `capture-manual-text`, and `inspect-item`.
- `gruenes-gewolbe-desktop`: desktop-shell-facing module that makes the active vault explicit, remembers multiple vault roots in user app state, and switches between them for future Tauri commands.
- Artwork saved items: local image files can be added to the Paintings subvault, preserved unchanged, recorded in Markdown, and reopened by stable item ID.
- Paintings import: supported image files can be copied from a local folder into the Paintings subvault through the core, CLI, and desktop shell.
- Metadata search: a derived index under `.gruenesgewolbe/` can be rebuilt from visible item records and queried through the core, CLI, and desktop shell.
- Workbench operations: the desktop shell can browse artwork grid items with rebuildable cached thumbnail previews, browse Idea Sources as a text list with source links, inspect item details, edit common item record fields, browse a cross-subvault review queue, suggest safer folder renames after metadata cleanup, and update review status while keeping item records canonical.
- Capture: source links can be saved as idea source items with extracted cleaned text or manual fallback text/image bundles, preserving source-link provenance, source copies, copied image bytes, saving reason, and review status.
- Organization: saved items can use a vault-level tag registry with aliases, durable collection files with item backreferences, and item links without changing their home subvault.
- Duplicate candidates: import and capture workflows record nonblocking duplicate warnings from local file fingerprints, source links, import provenance, and strong descriptive metadata.
- AI enrichment: captured ideas can be enriched through a provider boundary with visible budget modes, minimal cleaned-text context, accepted summaries/tags, staged metadata suggestions, additive better-file candidates that do not replace the primary file, image-only artwork metadata suggestion calls, and hidden AI cost log hooks. OpenAI provider config is stored in user-specific app state rather than the vault. Normal tests use fake providers; no live OpenAI call is required.
- Activity log and agent CLI: capture, import, metadata rebuild, enrichment, AI cost, and selected error events append to a hidden noncanonical activity log. The CLI exposes structured TSV-style output for validation, opening, import, rebuild, search, capture, item inspection, and selected errors.

The current test seam is the archive core public interface, with thin CLI and desktop shell tests around the same behavior.

## Verification

Run the test suite with:

```sh
cargo test
```

This repository expects a Rust toolchain with `cargo` and `rustc` available on `PATH`.

On systems without a C compiler on `PATH`, the current suite can be run through Rust's musl target and bundled linker:

```sh
RUSTFLAGS='-C linker=/home/cemkumral/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld -C link-self-contained=yes' \
  /home/cemkumral/.cargo/bin/cargo test --target x86_64-unknown-linux-musl
```
