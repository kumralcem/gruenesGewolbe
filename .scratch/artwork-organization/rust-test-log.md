# Rust verification

`scripts/check-bounded.sh cargo test --workspace --all-features`

Result: **PASS** (exit status 0)

All CLI, archive-core, desktop-library, Tauri-runtime, integration, and doc-test targets passed. The run executed 172 tests with zero failures.

Notable enrichment results:

- `artwork_enrichment::tests::*`: 6 passed, including durable resume, later paintings retained past the invocation item limit, option bounds, source allowlisting, and fingerprint-gated unchanged skips.
- `precomputed_artwork_enrichment_is_rejected_after_item_record_changes`: passed.
- `artwork_enrichment_guards_edits_and_preserves_user_title`: passed.
- Tauri command tests: 14 passed.

Bounded-run accounting:

- Service unit: `run-p68615-i71323.service`
- Service runtime: 2m 32.810s
- CPU time consumed: 2m 25.153s
- Memory peak: 2 GiB
- Swap peak: 0 B

No live provider calls were made during verification.

## Post-review focused rerun

After moving large-Vault metadata planning outside the global desktop lock:

- `cargo check -p gruenes-gewolbe-desktop --all-features`: PASS.
- `scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --all-features artwork_enrichment`: PASS.
- 6 enrichment module tests and the stale/user-edit command regression passed; 0 failed.
- Service runtime: 1m 15.504s; CPU: 1m 10.051s; memory peak: 2 GiB; swap: 0 B.
