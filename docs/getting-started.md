# Getting started

[Home](../README.md) · [Deployment](deployment.md) · [Capture guide](capture.md)

## Install

GG is a working prototype for a personal archive, with an MIT license. It currently targets a Linux controller and Chromium-based browsers (including Brave); there is no browser-store package or Firefox release. The controller can run on your own Linux machine or server.

For an **extension-only client**, clone the repository and load `extension/` as described below. No Node.js, pnpm, or Podman is needed on that client. You need access to a configured GG controller and a pairing code.

The controller requires Linux, Node.js 22+, pnpm and rootless Podman/crun. Browser and remote CLI clients do not need Podman. See [Deployment](deployment.md) for the service and connecting without a domain.

```sh
git clone https://github.com/kumralcem/gruenesGewolbe.git
cd gruenesGewolbe
pnpm install --frozen-lockfile
./scripts/install-cli.sh             # ~/.local/bin/gg; keep this checkout installed
pnpm build:worker
gg init --vault "$HOME/Gewolbe"      # requires an empty directory
```

Ensure `~/.local/bin` is on PATH. `init` saves the vault location, so daily commands need no `--vault` or `pnpm` prefix. You can choose initial destinations with `gg init Notes References --vault PATH`; without names the current defaults are Paintings, Photography, Sculptures, Ideas and Inbox. `init` creates a new vault; it is not a migration command.

## Choose a provider and sign in

Run sign-in on the controller host, under the same user/state directory as the service:

Choose **one** authentication method. For subscription sign-in:

```sh
gg login openai-codex
gg models openai-codex
gg configure --provider openai-codex --model YOUR_MODEL_ID
```

For API-key authentication, use `openai` or `openrouter` in all three commands:

```sh
gg login openai                     # prompts for the key without echoing it
gg models openai
gg configure --provider openai --model YOUR_MODEL_ID
```

Choose the model ID from the available catalog, then start the controller with `gg serve` (or install the [background service](deployment.md#run-as-a-service)).

`OPENAI_API_KEY` / `OPENROUTER_API_KEY` also work in the controller's environment. No automatic fallback to a paid API occurs. `gg provider PROVIDER MODEL` changes providers explicitly; for a running receiver, issue it through a paired management CLI, or restart after local configuration changes.

Credentials are in the controller's private state directory (`~/.config/gg`, overridden by `GG_STATE_DIR` or `--state-dir`), outside the vault. GG uses Pi's supported OAuth/login and refresh implementation, not another application's credential file. Account access to a model must still be validated by a live sign-in/request. OpenAI distinguishes [subscription sign-in and API-key authentication](https://developers.openai.com/codex/auth/); the account and provider determine actual availability.

## Connect the extension

1. On your browser machine, clone this repository. Open `chrome://extensions`, enable Developer mode, and **Load unpacked** → `extension/`.
2. On the controller, run `gg serve` and `gg pair`.
3. Open the extension’s **Settings & recent captures → Connect to GG**. Enter the server address and pairing code.
4. Open a page and click GG, or press **Alt+Shift+G**. Add instructions if useful, then **Enter** or **Capture**. **Shift+Enter** adds a line.

Pairing codes expire after ten minutes and are single-use. The resulting device connection survives restarts until revoked. To enable archive browsing/downloads as well as capture, pair with a code from `gg pair --scope manage` instead.

For a controller on the same machine, use `http://127.0.0.1:48123`. For a remote controller, see [connection options](deployment.md#remote-connections).

## Update

On the controller, from the installed checkout:

```sh
git pull --ff-only
pnpm install --frozen-lockfile
pnpm build:worker
systemctl --user restart gg          # if installed as a service
```

Do not rerun `gg init` on an existing vault. Schedule updates between jobs. The installed CLI uses the checkout, so keep it in place.

On a browser-only client, pull the checkout, click **Reload** on GG in the browser’s extensions page, and close old GG tabs. No Node.js or worker build is needed there. If the UI still looks old, check that the extension points to the checkout you updated.
