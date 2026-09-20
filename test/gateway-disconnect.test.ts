import { test, mock } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { Duplex } from "node:stream";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Vault } from "../src/vault.ts";
import { createGateway } from "../src/gateway.ts";

test("a broken pipe while rejecting CONNECT does not crash the controller", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-disconnect-"));
  const original = http.createServer;
  let server: http.Server;
  const spy = mock.method(http, "createServer", (...args: any[]) => {
    server = original(...args);
    return server;
  });
  const gateway = await createGateway({
    vault: await Vault.create(root, ["Ideas"]),
    intent: "ask",
    input: "test",
    config: { provider: "openrouter", model: "fixture" },
  });
  spy.mock.restore();
  try {
    // The peer has closed its read side: the actual CONNECT rejection write
    // completes asynchronously with the same EPIPE seen during the import.
    let rejection = "";
    const client = new Duplex({
      read() {},
      write(chunk, _encoding, callback) {
        rejection += chunk.toString();
        callback(Object.assign(new Error("write EPIPE"), { code: "EPIPE" }));
      },
    });
    server!.emit(
      "connect",
      { url: "example.com:443" },
      client,
      Buffer.alloc(0),
    );
    await new Promise((resolve) => setTimeout(resolve, 20));
    assert.match(rejection, /403 Forbidden/);
    assert.equal(client.destroyed, true);
    // The gateway must still accept the next worker request.
    const status = await new Promise((resolve, reject) => {
      const req = http.request(
        {
          socketPath: gateway.socket,
          path: "/search",
          method: "POST",
          headers: { authorization: `Bearer ${gateway.token}` },
        },
        (res) => {
          res.resume();
          res.on("end", () => resolve(res.statusCode));
        },
      );
      req.on("error", reject);
      req.end(JSON.stringify({ query: "test" }));
    });
    assert.equal(status, 200);
  } finally {
    await gateway.close();
    await rm(root, { recursive: true, force: true });
  }
});
