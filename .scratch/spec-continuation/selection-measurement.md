# Native image-selection measurement

## Baseline command evidence (2026-09-09)

Focused bounded command:

```sh
bash scripts/check-bounded.sh cargo test -p gruenes-gewolbe-desktop --features tauri-runtime --bin gruenes-gewolbe selection_stays_available_while_real_thumbnail_previews_are_prepared -- --nocapture
```

The deterministic fixture captured 64 separate Artwork Saved Items from the real
`workbench-desktop-linux.png` (100,053 bytes, 1440×1000), then exercised the real
`TauriCommandState` item-details, workbench-snapshot, and Thumbnail Preview paths.
The test observed the preparation worker holding the global command-state mutex
before issuing selection.

- first Item Details request after fixture creation: 106.1 ms
- repeat Item Details request: 99.1 ms
- full workbench snapshot selecting the same item: 2.084 s
- Thumbnail Preview preparation for 64 images: 24.988 s
- Item Details while preparation held the mutex: 24.988 s
- result: expected regression failure; selection returned only after preparation
  released the mutex
- bounded service: 1m 5.196s including compilation/setup; peak 2 GiB, zero swap

The first Item Details number is not a cold-disk measurement: fixture creation had
already populated filesystem caches. This is native Rust command/fixture evidence,
not a native window end-to-end measurement. It does not measure WebView click
feedback, preview protocol loading, browser image decode, or paint.

## Diagnosed boundary

`prepare_thumbnail_previews` held `Arc<Mutex<TauriCommandState>>` across Vault
scanning, source image decode, resizing, PNG encoding, derived writes, and possible
canonical Review Reason updates. `get_item_details` and `workbench_snapshot` acquire
the same mutex. The matched 24.988-second timings establish mutex wait as the
command-side delay when selection overlaps preparation. Independent of contention,
the lightweight Item Details command was about 20× faster than the full snapshot
on this 64-item fixture.

## Fix direction and safety constraints

Move scan/decode/derived Thumbnail Preview writes to an isolated Vault outside the
global command-state mutex. Keep canonical Item Record failure updates under the
mutex with an Active Vault root check, reject results after a Vault switch, and use
a separate single-worker gate so repeated native preparation commands cannot decode
in parallel. Preserve the existing 24-megapixel per-image decode limit.

## Evidence still missing

No native GUI is available in this environment. First/repeat click-to-feedback,
`vault-media` load, WebView decode, and paint need a native window acceptance run.
The issue must remain open until those boundaries are measured. A proposed native
criterion is visible selection feedback within 100 ms and populated Item Details
within 250 ms while a normal Thumbnail Preview batch is running; record the preview
load/paint interval separately.

## Post-fix command evidence

The same bounded regression, reduced to 16 fixture items to keep a focused rerun
short, passed after moving scan/decode/derived output outside the command-state
mutex:

- first Item Details request after fixture creation: 11.0 ms
- repeat Item Details request: 11.1 ms
- full workbench snapshot: 283.4 ms
- Thumbnail Preview preparation: 5.678 s
- Item Details while that preparation was active: 12.6 ms
- bounded service: 29.163 s including compilation; peak 2 GiB, zero swap

The regression synchronizes on the preparation helper after it snapshots the
Active Vault, then issues the real Item Details command while real PNG decode is
still active. Thumbnail preparation has its own process-local mutex, so native
invocations remain sequential. A narrow core callback keeps failed-preview Item
Record updates under the global command-state lock and checks both the Active Vault
root and current Primary File before committing. A stale result does not publish a
final placeholder, so reopening the original Vault can retry it.

The matched 64-item post-fix run passed together with the stale-Vault regression:

```sh
bash scripts/check-bounded.sh /usr/bin/env GG_SELECTION_ITEMS=64 cargo test -p gruenes-gewolbe-desktop --features tauri-runtime --bin gruenes-gewolbe selection_ -- --nocapture
```

- first Item Details request after fixture creation: 84.5 ms
- repeat Item Details request: 85.1 ms
- full workbench snapshot: 1.824 s
- Thumbnail Preview preparation: 23.305 s
- Item Details while that preparation was active: 148.7 ms
- bounded service: 56.425 s including compilation/setup; peak 2 GiB, zero swap

This compares directly with the 64-item baseline: overlapping selection fell from
24.988 s (the whole remaining preparation interval) to 148.7 ms while preparation
continued for 23.305 s. The first/repeat and snapshot values vary with filesystem
cache and host load; the regression signal is that selection completes before the
worker and no longer inherits its full duration. The separate damaged-image test
switches Active Vaults after preparation snapshots its root and verifies the old
Item Record receives no Review Reason and no final placeholder is published.

Artwork enrichment's per-item Thumbnail Preview preparation now uses the same
isolated preparation path and the same single-worker gate as gallery batches. This
keeps all image decode sequential while leaving Item Details available.
