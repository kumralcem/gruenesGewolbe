import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Devices } from "../src/devices.ts";
test("pairing is single-use, device credentials survive restart and can be revoked", async () => {
  const dir = await mkdtemp(join(tmpdir(), "gg-devices-"));
  const devices = new Devices(dir);
  const code = await devices.pairing("capture");
  const paired = await devices.exchange(code, "Laptop");
  await assert.rejects(devices.exchange(code, "Again"), /Invalid/);
  assert.equal(
    (await new Devices(dir).authenticate(paired.token))?.name,
    "Laptop",
  );
  assert.equal((await devices.authenticate(paired.token))?.scope, "capture");
  await devices.revoke(paired.id);
  assert.equal(await devices.authenticate(paired.token), undefined);
});
