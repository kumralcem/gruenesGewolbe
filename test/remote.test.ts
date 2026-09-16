import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { Devices } from "../src/devices.ts";
import { Controller } from "../src/controller.ts";
import { createReceiver } from "../src/receiver.ts";
import { connect, connection, remote, endpoint } from "../src/client.ts";
test("remote CLI pairing persists, scopes management, and revocation takes effect", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-remote-test-"));
  const vault = await Vault.create(join(root, "vault"), ["Ideas"]);
  const devices = new Devices(join(root, "server"));
  const controller = new Controller(
    vault,
    { provider: "openai", model: "fixture" },
    join(root, "server"),
  );
  const server = await createReceiver({
    vault,
    devices,
    port: 0,
    usage: () => controller.status(),
    command: (c, s) => controller.execute(c, s),
    processCapture: async () => ({ outcome: { status: "partial" } }),
  });
  try {
    const code = await devices.pairing("manage");
    const paired = await connect(join(root, "client"), server.url, code);
    const client = (await connection(join(root, "client")))!;
    assert.deepEqual(await remote(client, { command: "list" }), []);
    await assert.rejects(
      connect(join(root, "other"), server.url, code),
      /Invalid or expired/,
    );
    await devices.revoke(paired.id);
    await assert.rejects(
      remote(client, { command: "list" }),
      /Pair this device/,
    );
    const capture = await devices.exchange(server.token, "Browser");
    await assert.rejects(
      remote({ url: server.url, token: capture.token }, { command: "list" }),
      /management device/,
    );
    for (const url of [
      "http://private.example",
      "https://example.com/path",
      "https://user:secret@example.com",
    ]) {
      assert.throws(() => endpoint(url));
    }
    const headers = {
      authorization: "Bearer " + capture.token,
      "content-type": "application/json",
    };
    const accepted = await fetch(server.url + "/captures", {
      method: "POST",
      headers,
      body: JSON.stringify({
        version: 2,
        url: "https://private.example/post",
        title: "Partial",
        text: "Some content",
        capturedAt: "2026-09-16T00:00:00Z",
        images: [],
      }),
    });
    const job = (await accepted.json()) as { id: string };
    assert.equal(accepted.status, 202);
    await server.idle();
    assert.equal(
      (
        await fetch(server.url + `/captures/${job.id}/retry`, {
          method: "POST",
          headers,
        })
      ).status,
      202,
    );
    await server.idle();
    assert.equal(
      (
        await fetch(server.url + `/captures/${job.id}/cancel`, {
          method: "POST",
          headers,
        })
      ).status,
      200,
    );
    const cancelled = (await (
      await fetch(server.url + `/captures/${job.id}`, { headers })
    ).json()) as { status: string };
    assert.equal(cancelled.status, "cancelled");
  } finally {
    await server.close();
  }
});
