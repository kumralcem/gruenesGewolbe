# Review of the Pi prototype

Base: `c25f8f394519e38441cb264c9cef3e3df9e2bacd`. Initial reviewed commit: `3848a38`. Separate read-only agents reviewed Standards and Spec under the repository's review skill. Corrections were checked against the original findings; automated and container regressions were run by the implementation agent.

## Standards

No documented coding-standard violations or substantial architecture smells were found. Three correctness/security findings were raised and resolved:

- Provider request fields could expand paid operations beyond the intended function-tool loop. The gateway now admits an explicit set of Pi request fields, limits completion count, rejects provider-hosted tools and OpenRouter plugins/fallback/routing extensions, and prevents persistent/background requests. Regression tests cover these bypasses.
- Malformed JSON records could break search for all records. Stored records are validated, malformed entries are excluded, and their paths/reasons are reported on stderr while valid results remain available.
- Cancellation during container startup could treat “container not found” as successful cleanup. Failed removal now stops the launcher, waits for it to exit, and retries removal. A fake delayed runtime reproduces the startup race; a real-container test exercises a hung job followed by a successful job.

The Standards reviewer rechecked the fixes, including the initially missed OpenRouter plugin mechanism and ordinary CLI problem reporting, and found no remaining issue in those fixes.

## Spec

Three partial requirements were raised and resolved; no scope creep was found:

- Linked article text was dropped while retaining only the initial post. Explicitly included linked sources now preserve their URLs and complete fetched text alongside the submitted source.
- Submitting only an improved image could discard the initially downloaded copy. Capture now automatically includes the first matching normalized-image copy, even when the model only names an improvement.
- Retrieval could search beyond the first 60,000 source characters but could not inspect them. `archive_read` now supports bounded offsets, continuation, and total-length metadata.

The Spec reviewer inspected the fixes and regression assertions and found no remaining issue in those fixes within the prototype scope. Conservative image fingerprints, blocked public sources, unavailable live OpenRouter credentials, and deferred clean-CLI/GUI work remain disclosed limitations.

Standards: 3 findings resolved, highest initial severity P1. Spec: 3 findings resolved, highest initial severity P2.
