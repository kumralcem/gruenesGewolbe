# Extension validation

The single Chromium extension integration test passed in the existing Playwright worker image: 1 pass, 0 failures (approximately 9.5 seconds total).

The initial host attempt could not launch Chromium because `libnspr4.so` is missing. Container execution supplied the browser dependencies. The test needed the mapped controller UID, a writable temporary HOME for Chromium crash reporting, and temporary screenshot storage. No host packages or application code were changed.

Successful command, run from the repository:

```sh
podman run --rm --network=none --read-only --cap-drop=ALL \
  --userns=keep-id --user "$(id -u):$(id -g)" --env HOME=/tmp \
  --memory=2g --pids-limit=256 \
  --tmpfs=/tmp:rw,size=512m,mode=1777 \
  --tmpfs=/audit/.runs:rw,size=64m,mode=1777 \
  --volume /home/dev/projects/gruenesGewolbe:/audit:ro \
  --workdir /audit --entrypoint node localhost/gg-pi-prototype \
  --import tsx --test test/extension.integration.ts
```

This test uses a local authenticated fixture page and image. It is not a live browser-account or public HTTPS deployment test. The temporary screenshot volume disappears when the container exits. See [test output](extension-tests.log).
