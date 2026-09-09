# Capture routing review

Reviewed the current capture routing changes in:

- `crates/archive-core/src/lib.rs`
- `apps/desktop/src/lib.rs`
- `apps/desktop/src/main.rs`
- `apps/desktop/src/tauri-adapter.ts`
- `apps/desktop/tests/tauri_commands.rs`

## Result

Approved for the stated intent. No blocking bugs found in the scoped changes.

The generic core `Vault::capture_source_link` behavior remains intact: extracted
text is saved as an Idea Source and extracted images as Paintings. The desktop
Paintings command now wraps completed extraction so extracted text produces a
manual fallback prompt instead of creating an Idea Source. The new artwork
fallback validates that non-empty text is rejected and that a non-empty pasted
image is saved under Paintings. The existing manual fallback command still uses
the Idea Sources path. The Tauri command registration and adapter payload names
match, and the regression test covers text URL fallback, text-only rejection,
Paintings destination, and preserved image bytes.

One pre-existing concurrency limitation remains in the fallback flow: neither
manual fallback command carries an expected active-vault root, so a user who
switches Vaults after extraction and before submitting the fallback saves into
the Vault active at submit time. This is consistent with the legacy manual
fallback behavior and is outside the requested routing change; it is not a
release blocker for this review.

Validation: `git diff --check` passes for all scoped files. The targeted desktop
regression was reported passing by the parent agent.
