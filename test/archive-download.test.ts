import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, unlink, symlink } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { gunzipSync } from "node:zlib";
import { Vault } from "../src/vault.ts";
import { recordDownload } from "../src/archive-download.ts";
import { createReceiver } from "../src/receiver.ts";
import { Devices } from "../src/devices.ts";

function entries(compressed: Buffer) {
  const tar = gunzipSync(compressed),
    result = new Map<string, Buffer>();
  let offset = 0;
  while (tar[offset]) {
    const name = tar.toString("utf8", offset, offset + 100).split("\0")[0];
    const size = parseInt(tar.toString("ascii", offset + 124, offset + 136), 8);
    result.set(name, tar.subarray(offset + 512, offset + 512 + size));
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  return result;
}
async function bytes(stream: AsyncIterable<Buffer>) {
  const chunks = [];
  for await (const part of stream) chunks.push(part);
  return Buffer.concat(chunks);
}
test("portable exports preserve original bytes and Markdown links, exclude internal files and reject symlinks", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-download-"));
  const vault = await Vault.create(root, ["Art"]);
  const original = Buffer.from([255, 216, 255, 1, 2, 3]);
  await vault.capture(
    {
      kind: "art",
      title: "Example",
      summary: "An image",
      subvault: "Art",
      sourceUrl: "https://example.com/art",
      sourceText: "Source evidence",
      tags: [],
      assets: [
        {
          bytes: original.toString("base64"),
          width: 1,
          height: 1,
          visualHash: "a".repeat(64),
        },
      ],
    },
    "revision",
    "batch",
  );
  const [{ item, path }] = await vault.items();
  const bundle = entries(
    await bytes((await recordDownload(vault, item.id)).stream),
  );
  assert.deepEqual(bundle.get(item.assets[0].file), original);
  assert.deepEqual(
    bundle.get("record.md"),
    await readFile(join(path, "record.md")),
  );
  assert.equal(bundle.get("source.md")?.toString(), "Source evidence");
  assert.equal(
    [...bundle.keys()].some((k) => k.startsWith(".")),
    false,
  );
  const originals = entries(
    await bytes((await recordDownload(vault, item.id, true)).stream),
  );
  assert.deepEqual([...originals.keys()], [item.assets[0].file]);
  await unlink(join(path, item.assets[0].file));
  await symlink(join(path, "record.md"), join(path, item.assets[0].file));
  await assert.rejects(recordDownload(vault, item.id));
});

test("archive browsing and downloads require management pairing", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-download-http-"));
  const vault = await Vault.create(join(root, "vault"), ["Ideas"]);
  await vault.capture(
    {
      kind: "idea",
      title: "Instructions",
      subvault: "Ideas",
      summary: "Steps",
      sourceUrl: "https://example.com/steps",
      sourceText: "Complete steps",
      tags: [],
    },
    "revision",
    "batch",
  );
  const devices = new Devices(join(root, "state"));
  const receiver = await createReceiver({
    vault,
    devices,
    port: 0,
    processCapture: async () => ({}),
  });
  try {
    const capture = await devices.exchange(
      await devices.pairing("capture"),
      "capture",
    );
    const manage = await devices.exchange(
      await devices.pairing("manage"),
      "manage",
    );
    const auth = (token: string) => ({ authorization: `Bearer ${token}` });
    assert.equal((await fetch(receiver.url + "/archive")).status, 401);
    assert.equal(
      (await fetch(receiver.url + "/archive", { headers: auth(capture.token) }))
        .status,
      403,
    );
    const listing = await (
      await fetch(receiver.url + "/archive?q=Instructions", {
        headers: auth(manage.token),
      })
    ).json();
    assert.equal(listing.records.length, 1);
    const response = await fetch(
      receiver.url + `/archive/${listing.records[0].id}/download`,
      { headers: auth(manage.token) },
    );
    assert.equal(response.status, 200);
    assert.ok(
      entries(Buffer.from(await response.arrayBuffer())).has("record.md"),
    );
  } finally {
    await receiver.close();
  }
});
