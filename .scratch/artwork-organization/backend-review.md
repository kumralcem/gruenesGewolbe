# Artwork enrichment backend review

## Result

Approved. The prior skip finding is resolved and no blockers remain in the
reviewed backend/native loop.

`ArtworkEnrichmentCheckpoint::start` now carries forward a prior successful
item's record revision and fingerprint as a `skip_if_unchanged` candidate. The
native loop prepares and reads the current bounded Thumbnail Preview, marks the
item skipped only when its fingerprint still matches, and refreshes the stored
snapshot before enriching when it changed. This makes unchanged successful
paintings skippable across new runs while allowing changed thumbnails back into
the work queue.

## Review notes

The checkpoint model covers all supplied candidates, retains pending items for
later batches, and resumes pending work by run ID. The native loop is
single-flight, checks cancellation before each request, persists request counts
before the provider call, and uses a 90-second provider timeout. The provider
sends only the bounded Thumbnail Preview, uses `store: false`, and does not
include the API key in user-facing errors or persisted checkpoint data.
Monetary cost limits are intentionally unavailable after the UI/contract
cleanup; the backend correctly retains request and duration bounds. Factual
suggestions require URLs that appeared in actual web-search sources;
unsupported source URLs are dropped and visual-only provenance is used for
descriptive suggestions.

The core `apply_artwork_enrichment_response` seam checks the expected Item
Record revision before writing, preserving the required stale-record guard.
The later startup-planning lock refinement was reviewed separately by the root agent.
