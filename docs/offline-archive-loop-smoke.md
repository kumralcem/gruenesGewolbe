# Offline Archive Loop Linux Smoke Run

This checklist verifies the real Tauri window and canonical filesystem behavior. Run it on Linux with networking disabled; no OpenAI credentials or external services are required.

## Fixtures

Prepare a temporary empty Vault folder and an import folder containing:

- a valid JPEG or PNG at the top level;
- a different valid image in a nested folder;
- a byte-identical copy of the first image;
- two differently encoded images whose filenames and supplied metadata strongly overlap, to produce a Duplicate Candidate;
- an undecodable file with a supported image extension, such as `damaged.jpg` containing plain text;
- a valid supported image made unreadable with `chmod 000 unreadable.png` after creating it, to exercise a per-file Import Run failure (restore its permissions before cleanup);
- an unsupported text file; and
- a symbolic link to a file or folder.

## Run

1. Run `./run.sh` and create the Vault through the native folder dialog. Confirm the empty Paintings workbench appears.
2. Add one valid image with **Add Artwork**. Confirm the Preserved File has identical bytes and a rendered Thumbnail Preview appears.
3. Import the fixture folder. Confirm nested files are discovered and the summary distinguishes imported, unsupported, symbolic-link, Exact File Duplicate, Duplicate Candidate, and failed outcomes. In the gallery, confirm the undecodable image uses a placeholder and Item Details shows its Thumbnail Preview Review Reason; confirm the Activity Log records the decode failure.
4. Start the import again and cancel after progress appears. Confirm completed Saved Items remain and the summary reports cancellation.
5. Browse and sort Paintings, search for imported metadata, and open the result in Item Details.
6. Edit title, creator, year, Saving Reason, summary, and Tags, then save. Resolve each Review Reason independently; confirm Review Status becomes `reviewed` only after the final resolution.
7. Move one Saved Item to Vault Trash. Confirm it disappears from normal browsing/search and appears under the correct Home Subvault in Vault Trash; restore it and confirm it returns.
8. Close the application, run `./run.sh` again, and confirm the same Active Vault and canonical state reopen.

## Recovery cases

1. While the app is closed, replace one Item Record with malformed YAML and retain a byte-for-byte copy of it.
2. Relaunch. Confirm valid Saved Items remain browseable/searchable and Vault Problems shows the malformed record path and parse error. Confirm the malformed bytes are unchanged.
3. Correct that record externally and use **Refresh Item Records**. Confirm the Vault Problem disappears and the Saved Item returns to browse/search.
4. Close the app, remove one safe structural directory from the Vault, and relaunch/open it. Confirm the repair proposal lists only the missing directory, cancellation makes no changes, and explicit confirmation recreates it without rewriting Item Records.

## Evidence to retain

- terminal output from `./run.sh` and `./build.sh`;
- the release binary path printed by `./build.sh`;
- screenshots of the desktop and constrained browser workbench runs;
- Import Run summaries for duplicate, failure, and cancellation cases;
- before/after hashes for a Preserved File and the malformed Item Record; and
- the generated personal-scale test timings printed by `cargo test -p gruenes-gewolbe-core --test personal_scale -- --nocapture`.

Automated native-window WebDriver coverage is intentionally deferred. Reconsider it when recurring native dialog, window lifecycle, or Tauri integration regressions make the maintenance cost worthwhile.
