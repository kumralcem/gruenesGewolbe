import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { once } from "node:events";
import http from "node:http";
import net from "node:net";
import dns from "node:dns/promises";
import { syncBuiltinESMExports } from "node:module";
import { Vault } from "../src/vault.ts";
import { createGateway } from "../src/gateway.ts";
import type { Intent } from "../src/types.ts";

type Gateway = Awaited<ReturnType<typeof createGateway>>;
function rpc(g: Gateway, path: string, body: unknown) {
  return new Promise<number>((resolve, reject) => {
    const req = http.request(
      {
        socketPath: g.socket,
        path,
        method: "POST",
        headers: { authorization: `Bearer ${g.token}` },
      },
      (res) => {
        res.resume();
        res.on("end", () => resolve(res.statusCode!));
      },
    );
    req.on("error", reject);
    req.end(JSON.stringify(body));
  });
}
function tunnel(g: Gateway, target = "93.184.216.34:443") {
  return new Promise<{ status: number; socket: net.Socket }>(
    (resolve, reject) => {
      const req = http.request({
        socketPath: g.socket,
        method: "CONNECT",
        path: target,
      });
      req.on("connect", (res, socket) =>
        resolve({ status: res.statusCode!, socket }),
      );
      req.on("error", reject);
      req.end();
    },
  );
}

test(
  "network capability applies to both transports and closes existing tunnels before private policy",
  { timeout: 15000 },
  async () => {
    const root = await mkdtemp(join(tmpdir(), "gg-network-capability-"));
    const vault = await Vault.create(root, ["Inbox"]);
    const publicServer = net.createServer((socket) => socket.resume());
    publicServer.listen(0, "127.0.0.1");
    await once(publicServer, "listening");
    const originalConnect = net.connect;
    const originalLookup = dns.lookup;
    let connections = 0;
    const gateways: Gateway[] = [];
    try {
      net.connect = (() => {
        connections++;
        return originalConnect({
          host: "127.0.0.1",
          port: (publicServer.address() as net.AddressInfo).port,
        });
      }) as typeof net.connect;
      syncBuiltinESMExports();
      const make = async (intent: Intent, snapshot = false) => {
        const gateway = await createGateway({
          vault,
          intent,
          input: "https://source.example/",
          config: { provider: "openai", model: "fixture" },
          browserCapture: snapshot
            ? {
                version: 2,
                intent: "capture",
                url: "https://source.example/",
                title: "Private",
                text: "Private",
                images: [],
                capturedAt: new Date().toISOString(),
              }
            : undefined,
          fixtureFetch: async (url) => ({
            url,
            type: "text/plain",
            bytes: Buffer.from("public"),
          }),
        });
        gateways.push(gateway);
        return gateway;
      };
      for (const intent of [
        "ask",
        "manage",
        "capture",
        "art",
        "idea",
        "probe",
      ] as const) {
        const g = await make(intent);
        const allowed = intent !== "ask" && intent !== "manage";
        const health = await rpc(g, "/probe", {});
        if (intent === "probe") assert.equal(health, 200);
        else assert.notEqual(health, 200);
        assert.equal(
          await rpc(g, "/fetch", { url: "https://example.org" }),
          allowed ? 200 : 400,
          intent,
        );
        const t = await tunnel(g);
        assert.equal(t.status, allowed ? 200 : 403, intent);
        t.socket.destroy();
        await g.close();
        gateways.pop();
      }
      for (const intent of ["capture", "art", "idea"] as const) {
        const g = await make(intent, true);
        const before = connections;
        assert.equal(
          await rpc(g, "/fetch", { url: "https://example.org" }),
          400,
        );
        const t = await tunnel(g);
        assert.equal(t.status, 403);
        t.socket.destroy();
        assert.equal(connections, before);
        await g.close();
        gateways.pop();
      }
      const g = await make("capture");
      const t = await tunnel(g);
      assert.equal(t.status, 200);
      t.socket.resume();
      const closed = once(t.socket, "close");
      assert.equal(await rpc(g, "/capture-policy", { subvault: "Inbox" }), 200);
      await closed;
      assert.equal(t.socket.destroyed, true);
      const blocked = await tunnel(g);
      assert.equal(blocked.status, 403);
      blocked.socket.destroy();

      // A CONNECT request admitted before policy must recheck after DNS resolves.
      const delayed = await make("capture");
      let resolveDns!: (
        addresses: { address: string; family: number }[],
      ) => void;
      let started!: () => void;
      const lookupStarted = new Promise<void>((resolve) => {
        started = resolve;
      });
      dns.lookup = (() => {
        started();
        return new Promise((resolve) => {
          resolveDns = resolve;
        });
      }) as typeof dns.lookup;
      syncBuiltinESMExports();
      const before = connections;
      const pending = tunnel(delayed, "pending.example:443");
      await lookupStarted;
      assert.equal(
        await rpc(delayed, "/capture-policy", { subvault: "Inbox" }),
        200,
      );
      resolveDns([{ address: "93.184.216.34", family: 4 }]);
      const denied = await pending;
      assert.equal(denied.status, 403);
      denied.socket.destroy();
      assert.equal(connections, before);
    } finally {
      net.connect = originalConnect;
      dns.lookup = originalLookup;
      syncBuiltinESMExports();
      for (const gateway of gateways) await gateway.close();
      await new Promise<void>((resolve) => publicServer.close(() => resolve()));
      await rm(root, { recursive: true, force: true });
    }
  },
);

test(
  "private policy aborts an in-flight public fetch without returning its response",
  { timeout: 15000 },
  async () => {
    const root = await mkdtemp(join(tmpdir(), "gg-fetch-revoke-"));
    const vault = await Vault.create(root, ["Inbox"]);
    let started!: () => void;
    const receiving = new Promise<void>((resolve) => {
      started = resolve;
    });
    let disconnected!: () => void;
    const ended = new Promise<void>((resolve) => {
      disconnected = resolve;
    });
    const source = http.createServer((_req, res) => {
      res.writeHead(200, { "content-type": "text/plain" });
      res.write("partial response");
      res.on("close", disconnected);
      started();
    });
    source.listen(0, "127.0.0.1");
    await once(source, "listening");
    const originalGet = http.get;
    const gateway = await createGateway({
      vault,
      intent: "capture",
      input: "http://93.184.216.34/",
      config: { provider: "openai", model: "fixture" },
    });
    try {
      // Only the public fetch is intercepted; gateway RPC uses http.request.
      http.get = ((
        _url: unknown,
        options: http.RequestOptions,
        callback: (res: http.IncomingMessage) => void,
      ) =>
        originalGet(
          `http://127.0.0.1:${(source.address() as net.AddressInfo).port}`,
          { ...options, lookup: undefined },
          callback,
        )) as typeof http.get;
      const pending = rpc(gateway, "/fetch", { url: "http://93.184.216.34/" });
      await receiving;
      assert.equal(
        await rpc(gateway, "/capture-policy", { subvault: "Inbox" }),
        200,
      );
      assert.equal(await pending, 400);
      await ended;
    } finally {
      http.get = originalGet;
      await gateway.close();
      source.closeAllConnections();
      await new Promise<void>((resolve) => source.close(() => resolve()));
      await rm(root, { recursive: true, force: true });
    }
  },
);
