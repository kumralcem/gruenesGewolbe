import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  symlink,
  readFile,
  rm,
} from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import sharp from "sharp";
import { imageCapture, imageFiles, importId } from "../src/image-import.ts";
import { validateBrowserCapture } from "../src/browser-capture.ts";
import { Vault } from "../src/vault.ts";
import { createReceiver } from "../src/receiver.ts";
import { importRemote } from "../src/client.ts";

test("image import scans recursively, excludes links, validates provenance and resumes by content", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-import-test-"));
  let receiver: Awaited<ReturnType<typeof createReceiver>> | undefined;
  try {
    const input = join(root, "input");
    await mkdir(join(input, "nested"), { recursive: true });
    const bytes = await sharp({
      create: { width: 16, height: 16, channels: 3, background: "green" },
    })
      .png()
      .toBuffer();
    await writeFile(join(input, "one.png"), bytes);
    await writeFile(join(input, "nested", "copy.png"), bytes);
    await writeFile(join(input, "notes.txt"), "not an image");
    await symlink(input, join(input, "loop"));
    const files = [];
    for await (const file of imageFiles([input])) files.push(file);
    assert.equal(files.length, 2);
    const capture = await imageCapture(files[0], "Save under Art");
    assert.equal(
      importId(capture),
      importId({ ...capture, capturedAt: new Date(0).toISOString() }),
    );
    assert.throws(
      () =>
        validateBrowserCapture({
          ...capture,
          url: "gg-local:sha256:" + "0".repeat(64),
        }),
      /image|reference/i,
    );
    assert.throws(
      () => validateBrowserCapture({ ...capture, url: "file:///etc/passwd" }),
      /URL/,
    );
    await assert.rejects(async () => {
      for await (const _ of imageFiles([join(input, "loop")])) {
      }
    }, /links/);
    const vault = await Vault.create(join(root, "vault"), ["Art"]);
    let calls = 0;
    let failNext = false;
    receiver = await createReceiver({
      vault,
      port: 0,
      processCapture: async (c) => {
        calls++;
        if (failNext) {
          failNext = false;
          return {
            outcome: { status: "failed", reason: "fixture interruption" },
          };
        }
        return {
          outcome: await vault.capture(
            {
              kind: "art",
              title: c.title,
              summary: "Green image",
              subvault: "Art",
              tags: [],
              sourceUrl: c.url,
              captureKey: "source",
              sourceText: c.text,
              assets: [
                {
                  bytes: c.images![0].bytes!,
                  width: 16,
                  height: 16,
                  visualHash: "a".repeat(64),
                },
              ],
            },
            "a".repeat(64),
            "import-test",
          ),
        };
      },
    });
    const conn = { url: receiver.url, token: receiver.token };
    const first = await importRemote(conn, capture, importId(capture));
    assert.equal(first.outcome.status, "saved");
    const again = await imageCapture(files[0], "Save under Art");
    await importRemote(conn, again, importId(again));
    const renamed = await imageCapture(files[1]);
    await importRemote(conn, renamed, importId(renamed));
    assert.equal(calls, 1, "duplicates must not invoke the model again");
    const state = join(root, "client");
    await mkdir(state);
    await writeFile(join(state, "client.json"), JSON.stringify(conn), {
      mode: 0o600,
    });
    const { execFile } = await import("node:child_process");
    const { promisify } = await import("node:util");
    const cli = await promisify(execFile)(process.execPath, [
      "--import",
      "tsx",
      "src/cli.ts",
      "import",
      input,
      "--state-dir",
      state,
      "--json",
    ]);
    assert.equal(JSON.parse(cli.stdout).outcome.status, "existing");
    assert.equal(calls, 1);

    const records = await vault.sourceRecords(capture.url);
    assert.equal(records.length, 1);
    const nextPath = join(input, "next.png");
    await writeFile(
      nextPath,
      await sharp({
        create: { width: 16, height: 16, channels: 3, background: "red" },
      })
        .png()
        .toBuffer(),
    );
    const next = await imageCapture(nextPath);
    failNext = true;
    await assert.rejects(
      importRemote(conn, next, importId(next)),
      /fixture interruption/,
    );
    const retried = await imageCapture(nextPath);
    assert.equal(
      (await importRemote(conn, retried, importId(retried))).outcome.status,
      "saved",
    );
    assert.equal(calls, 3);

    const asset = records[0].item.assets[0];
    assert.deepEqual(await readFile(join(records[0].path, asset.file)), bytes);
    assert.match(
      await readFile(join(records[0].path, "record.md"), "utf8"),
      /Imported local image/,
    );
  } finally {
    await receiver?.close();
    await rm(root, { recursive: true, force: true });
  }
});

test("controller accepts validated local images while retaining usage pauses and rejecting bare local URLs", async () => {
  const { Controller } = await import("../src/controller.ts");
  const root = await mkdtemp(join(tmpdir(), "gg-import-controller-"));
  try {
    const file = join(root, "large.png");
    const png = await sharp({
      create: { width: 16, height: 16, channels: 3, background: "green" },
    })
      .png()
      .toBuffer();
    await writeFile(file, Buffer.concat([png, Buffer.alloc(31_000_000)]));
    const capture = await imageCapture(file);
    assert.ok(
      Buffer.from(capture.images![0].bytes!, "base64").length > 30_000_000,
    );
    const c = new Controller(
      await Vault.create(join(root, "vault"), ["Art"]),
      { provider: "openai", model: "fixture" },
      join(root, "state"),
    );
    await c.usage.failure("openai", "test pause", true);
    assert.equal(
      (await c.run("capture", capture.url, undefined, capture)).outcome?.status,
      "paused",
    );
    await assert.rejects(c.run("capture", capture.url), /HTTP/);
    await assert.rejects(
      c.run("capture", capture.url, undefined, { ...capture, images: [] }),
      /original image/,
    );
    const { truncate } = await import("node:fs/promises");
    await truncate(file, 64_000_001);
    await assert.rejects(imageCapture(file), /64 MB/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
