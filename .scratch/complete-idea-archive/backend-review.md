# Backend review against `fd5b602`

Reviewed 2026-09-07, read-only. No tests, builds, or browser commands were run.

## Finding

### High: HTTPS redirects bypass the image host allow-list

`apps/desktop/src/source_extractor.rs:22-28` configures a custom redirect policy that follows any redirect whose destination is HTTPS, has no username, and is within four hops (`should_follow_redirect` at lines 240-243). `download_image` then checks the initial URL with `is_allowed_image_host` (lines 139-144), but does not re-check the final URL after redirects. The response is accepted solely because its `Content-Type` starts with `image/` (lines 155-162).

Consequently an allowed Wikimedia or `pbs.twimg.com` image URL can redirect to any HTTPS host and the extractor will download and preserve that host's bytes. This defeats the stated supported-media-host boundary and permits an attacker controlling a redirect (or a compromised/intermediary endpoint) to make the app fetch arbitrary HTTPS resources. The same unrestricted redirect policy also lets a generic Idea Source URL cross to an unrelated HTTPS host, and can reach private HTTPS endpoints if DNS/routing permits.

The policy should retain the original/allowed-host context and reject a redirect unless its destination is in the permitted host set for that operation (and ideally reject loopback/private/link-local IP destinations after DNS resolution). At minimum, revalidate `response.url()` before accepting an image; the redirect policy must also prevent downloading the body from the disallowed hop because rejecting only after transfer still permits the network request.

## Reviewed areas

- Provider API calls keep network I/O outside the global command-state lock: `summarize_live` snapshots source/config under the lock, calls OpenAI, then reacquires the lock to verify the source is unchanged.
- Provider response and source/image bodies are bounded; API-key status is redacted, and provider config is written with Unix mode `0600` (the existing direct truncate write is crash-tearable but not an access-control issue).
- Idea-source reads reject absolute paths, parent components, and canonical paths outside the item folder (`crates/archive-core/src/lib.rs:3972-4005`), so the new source-copy path handling did not reveal a traversal issue.
- Active-vault checks around source capture are performed again while holding command state before mutation, which closes the vault-switch race in that path.
