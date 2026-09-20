# Local image import

Implemented `gg import PATH... --instructions TEXT` using existing snapshot transport, durable server queue, gateway and isolated Pi worker. Direct local mode processes sequentially with the controller job lock.

Local provenance is `gg-local:sha256:<hash>`, not a fake web URL or accessible file path. Snapshot validation binds the single supplied image to this hash; the gateway requires its exact bytes in saved assets. Network access rules remain unchanged. The original filename is context, never verified attribution.

Completed records deduplicate by source hash. Stable upload IDs omit capture timestamp; retained first upload remains authoritative on reconnect. Failed jobs retry through the existing queue. Deleted/undone records can be imported again. Per-file results are printed; a failed/partial/paused job stops further uploads. Existing records require separate management commands for changed instructions.

Supported: JPEG/PNG/WebP, 20 MB/file, recursive nonhidden files, no symlinks. No general directory reconciliation or bulk document import.
