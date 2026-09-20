<p align="center"><img src="assets/brand/logo.svg" width="112" alt="Grünes Gewölbe logo" /></p>

# Grünes Gewölbe — GG

A personal archive you can capture into, search and organize with an agent. Save a page from your browser, optionally add instructions, and GG keeps the useful content in ordinary Markdown files alongside original files.

- **Capture what you’re reading**, including pages available through your browser login.
- **Organize in folders**, with shared and per-folder `CAPTURE.md` instructions.
- **Search and manage conversationally** through the CLI.
- **Keep portable files** you can open directly or in Obsidian.

GG uses Pi for its agent runtime. A controller holds your archive and provider credentials; isolated workers process jobs. The browser extension and CLI connect to that controller.

**Prototype:** Linux controller, Chromium-based extension. Local imports support common UTF-8 text formats and JPEG, PNG or WebP images. See [supported inputs and limitations](docs/files.md#supported-inputs).

## Get started

The controller needs Node.js 22+, pnpm and rootless Podman/crun:

```sh
git clone https://github.com/kumralcem/gruenesGewolbe.git
cd gruenesGewolbe
pnpm install --frozen-lockfile
./scripts/install-cli.sh
pnpm build:worker
gg init Notes References --vault "$HOME/Gewolbe"
```

Then [configure a provider and connect the extension](docs/getting-started.md). A browser-only client just needs this repository’s `extension/` folder and a paired controller—no Node.js or Podman installation.

## Everyday use

Click the extension or press **Alt+Shift+G**, add instructions if needed, and capture. GG chooses a destination; you can refine its behavior in settings.

```sh
gg search "release checklist"
gg ask "What did I save about onboarding?"
gg do "Move the onboarding checklist into Notes"
gg usage
```

## Documentation

| Guide                                       | Covers                                                           |
| ------------------------------------------- | ---------------------------------------------------------------- |
| [Getting started](docs/getting-started.md)  | Installation, provider login, pairing and updates                |
| [Capture and organization](docs/capture.md) | Browser capture, folder instructions, editing and undo           |
| [Import and export](docs/files.md)          | Supported formats, uploads, downloads and current specialization |
| [CLI reference](docs/cli.md)                | Commands, usage limits and temporary allowances                  |
| [Deployment](docs/deployment.md)            | Services, remote connections and archive copies                  |
| [Development](docs/development.md)          | Tests, architecture and contributing assets                      |
| [Known limits](docs/validation.md)          | Validation coverage and remaining reliability work               |

Named after Dresden’s Grünes Gewölbe—a treasury for things worth keeping.

[MIT license](LICENSE)
