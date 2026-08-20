# Gruenes Gewoelbe

A local-first personal archive for saving, describing, and rediscovering material found while browsing.

## Offline Archive Loop

The Linux desktop workbench implements the local, file-first archive workflow: create or open a format-v2 Vault, add selected artwork files, recursively import a Paintings folder, browse real Thumbnail Previews, search and edit Item Records, resolve Review Reasons, inspect localized Vault Problems, and move or restore Saved Items through visible Vault Trash. The last valid Active Vault is remembered outside the Vault and reopened on restart.

The implementation remains usable without network access, credentials, or a background service. Canonical Vault files stay authoritative; derived previews and metadata indexes can be rebuilt.

The main implementation surfaces are:

- `gruenes-gewolbe-core`: archive core module for creating, opening, and validating a vault.
- `ggvault`: structured CLI adapter for Vault lifecycle, selected-file and folder import, search, Item Record inspection, Vault Problems, duplicate resolution, and Vault Trash operations.
- `gruenes-gewolbe-desktop`: a Tauri 2 Linux desktop application with native folder dialogs, explicit active-vault handling, remembered vault roots, and a TypeScript/Vite workbench.
- Artwork saved items: local image files can be added to the Paintings subvault, preserved unchanged, recorded in Markdown, and reopened by stable item ID.
- Paintings import: supported image files can be copied from a local folder into the Paintings subvault through the core, CLI, and desktop shell.
- Metadata search: a derived index under `.gruenesgewolbe/` can be rebuilt from visible item records and queried through the core, CLI, and desktop shell.
- Workbench operations: the desktop shell can build a first-screen workbench snapshot with active vault, subvault navigation, collections, artwork grid items, Idea Sources, Review Queue, search results, and selected item details. It can also browse artwork grid items with rebuildable cached Thumbnail Previews, inspect item details, edit common Item Record fields, suggest safer folder renames after metadata cleanup, and accept, correct, or dismiss individual Review Reasons while deriving Review Status automatically.
- Thumbnail preparation: uncached previews are generated one at a time in background work after the gallery becomes usable. Sources above the safe 24-megapixel decode limit receive a placeholder and Review Reason instead of monopolizing memory or freezing the window; cached previews are reused on later launches.
- Capture foundations: archive-core retains earlier source-extraction, manual fallback, and AI-provider boundaries for future work, but the Offline Archive Loop workbench deliberately exposes no URL Capture or AI Enrichment controls.
- Organization: saved items can use a vault-level tag registry with aliases, durable collection files with item backreferences, and item links without changing their home subvault.
- Duplicate candidates: import and capture workflows record nonblocking duplicate warnings from local file fingerprints, source links, import provenance, and strong descriptive metadata.
- Deferred work: URL Capture, live AI Enrichment, installers/AppImage, automatic updates, automated native-window WebDriver testing, and verified Windows/macOS support are not part of this milestone.
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

The manual Linux acceptance path, edge-case fixtures, restart checks, and expected evidence are documented in [the Offline Archive Loop smoke run](docs/offline-archive-loop-smoke.md). Native-window WebDriver automation remains deferred until native integration regressions justify its maintenance cost.

On systems without a C compiler on `PATH`, the current suite can be run through Rust's musl target and bundled linker:

```sh
RUSTFLAGS='-C linker=/home/cemkumral/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld -C link-self-contained=yes' \
  /home/cemkumral/.cargo/bin/cargo test --target x86_64-unknown-linux-musl
```
