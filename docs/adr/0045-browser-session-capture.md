---
status: accepted
---

# Browser Session Capture

On 2026-09-11 the user explicitly requested a browser extension which uses their own browser session to obtain page content, HTML, or relevant media for the agent. Chrome/Chromium is the selected first browser. This supersedes ADR-0040's earlier public-only restriction for this user-initiated capture path. Direct CLI URL fetching remains unauthenticated.

An explicit toolbar action or shortcut collects visible text, sanitized HTML, an already-open transcript, and selectable images from the active tab. The user chooses idea capture or exactly one image. Image bytes are obtained inside the user's browser, with an optional explicit host-permission request when cross-origin downloading needs it. Chrome's [activeTab permission](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab) scopes initial page access to the user's gesture; [extension network rules](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests) govern the image fallback.

The extension sends a versioned snapshot to a paired local GG receiver. It does not export browser cookies, credential stores, storage, form contents, scripts, or authentication headers. Page content itself can be private and is processed by the configured model provider. This is content capture, not an offline whole-site mirror.

The receiver binds only loopback and checks a random per-session token, Host, and Origin. Inputs and job state are ordinary files in `.gg-jobs`. Jobs continue after the browser tab closes; pending work resumes after restart, while interrupted work requires explicit retry. Idempotency IDs prevent accidental duplicate submission. Input, queue, and retained-byte limits bound accumulation.

The worker receives snapshots through its existing gateway. It can inspect the supplied page/image without trying to re-fetch an inaccessible public version. Provider keys, authenticated browser profiles, and unrestricted vault access remain outside the worker. User uncertainty stays in the distinct Capture Queue.

This local receiver is not a hosted deployment. Firefox, remote pairing, and live-archive migration remain separate work.
