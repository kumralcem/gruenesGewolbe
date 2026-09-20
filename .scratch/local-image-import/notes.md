# Local image import

Implemented `gg import PATH... --instructions TEXT` using existing snapshot transport, durable server queue, gateway and isolated Pi worker. Direct local mode processes sequentially with the controller job lock.

Local provenance is `gg-local:sha256:<hash>`, not a fake web URL or accessible file path. Snapshot validation binds the single supplied image to this hash; the gateway requires its exact bytes in saved assets. Network access rules remain unchanged. The original filename is context, never verified attribution.

Completed records deduplicate by source hash. Stable upload IDs omit capture timestamp; retained first upload remains authoritative on reconnect. Failed jobs retry through the existing queue. Deleted/undone records can be imported again. Per-file results are printed; a failed/partial/paused job stops further uploads. Existing records require separate management commands for changed instructions.

Supported: JPEG/PNG/WebP, 64 MB/file, recursive nonhidden files, no symlinks. No general directory reconciliation or bulk document import.

Follow-up: Controller.run still enforced HTTP(S) URLs, which blocked real imports before worker launch despite lower-level tests passing. It now admits content-addressed sources only with a matching, validated supplied snapshot. The new controller regression test reproduced that failure before the fix and also checks usage pauses, missing-image rejection, >20 MB acceptance, and the 64 MB ceiling. Normal URL capture still requires HTTP(S).
