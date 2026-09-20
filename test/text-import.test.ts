import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { gunzipSync } from "node:zlib";
import { fileCapture, importFiles, importId } from "../src/image-import.ts";
import { validateBrowserCapture } from "../src/browser-capture.ts";
import { Vault } from "../src/vault.ts";
import { recordDownload } from "../src/archive-download.ts";
import { createReceiver } from "../src/receiver.ts";
import { importRemote } from "../src/client.ts";

test("text import preserves UTF-8 BOM, CRLF and originals through remote resume and export", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-text-"));
  let receiver: Awaited<ReturnType<typeof createReceiver>> | undefined;
  try {
    const original = Buffer.from(
      "\ufeff---\r\ntitle: Notes\r\n---\r\n# Grünes Gewölbe\r\n\r\n1. First tip\r\n2. Second tip\r\n",
    );
    const file = join(root, "notes.md");
    await writeFile(file, original);
    const capture = await fileCapture(file);
    assert.equal(capture.document?.extension, "md");
    assert.deepEqual(Buffer.from(capture.text), original);
    assert.throws(
      () => validateBrowserCapture({ ...capture, text: "tampered" }),
      /hash/,
    );
    assert.throws(
      () =>
        validateBrowserCapture({
          ...capture,
          document: { extension: "../secret" },
        }),
      /format/,
    );
    assert.throws(
      () => validateBrowserCapture({ ...capture, url: "https://example.com" }),
      /document/,
    );
    const vault = await Vault.create(join(root, "vault"), ["Notes"]);
    let calls = 0;
    receiver = await createReceiver({
      vault,
      port: 0,
      processCapture: async (c) => {
        calls++;
        return {
          outcome: await vault.capture(
            {
              kind: "idea",
              title: c.title,
              summary: "Two tips",
              subvault: "Notes",
              sourceUrl: c.url,
              tags: [],
              sourceText: c.text,
              originalFile: "original.md",
            },
            "revision",
            "batch",
          ),
        };
      },
    });
    const conn = { url: receiver.url, token: receiver.token };
    assert.equal(
      (await importRemote(conn, capture, importId(capture))).outcome.status,
      "saved",
    );
    const renamed = { ...capture, title: "renamed.md" };
    assert.equal(
      (await importRemote(conn, renamed, importId(renamed))).outcome.status,
      "existing",
    );
    const { execFile } = await import("node:child_process");
    const { promisify } = await import("node:util");
    const { mkdir } = await import("node:fs/promises");
    const state = join(root, "client");
    await mkdir(state);
    await writeFile(join(state, "client.json"), JSON.stringify(conn));
    const cli = await promisify(execFile)(process.execPath, [
      "--import",
      "tsx",
      "src/cli.ts",
      "import",
      file,
      "--state-dir",
      state,
      "--json",
    ]);
    assert.equal(JSON.parse(cli.stdout).outcome.status, "existing");
    assert.equal(calls, 1);
    const [record] = await vault.items();
    assert.deepEqual(
      await readFile(join(record.path, "original.md")),
      original,
    );
    assert.deepEqual(await readFile(join(record.path, "source.md")), original);
    assert.match(
      await readFile(join(record.path, "record.md"), "utf8"),
      /Original document.*original.md/,
    );
    const chunks = [];
    for await (const c of (await recordDownload(vault, record.item.id, true))
      .stream)
      chunks.push(c);
    const tar = gunzipSync(Buffer.concat(chunks));
    assert.equal(tar.toString("utf8", 0, 100).split("\0")[0], "original.md");
    assert.deepEqual(tar.subarray(512, 512 + original.length), original);
  } finally {
    await receiver?.close();
    await rm(root, { recursive: true, force: true });
  }
});

test("mixed file discovery and text validation reject unsupported/binary/oversized input", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-text-formats-"));
  try {
    for (const ext of [
      "md",
      "markdown",
      "txt",
      "rst",
      "csv",
      "tsv",
      "json",
      "yaml",
      "yml",
      "toml",
      "log",
    ]) {
      const file = join(root, `notes.${ext}`);
      await writeFile(file, "Plain text\n");
      assert.equal((await fileCapture(file)).document?.extension, ext);
    }
    await writeFile(join(root, "skip.pdf"), "unsupported");
    const paths = [];
    for await (const file of importFiles([root])) paths.push(file);
    assert.equal(paths.length, 11);
    const file = join(root, "bad.txt");
    for (const bytes of [
      Buffer.from([0xff]),
      Buffer.from("abc\0def"),
      Buffer.from("   "),
      Buffer.alloc(64001, 65),
    ]) {
      await writeFile(file, bytes);
      await assert.rejects(fileCapture(file));
    }
    await assert.rejects(async () => {
      for await (const _ of importFiles([join(root, "skip.pdf")])) {
      }
    }, /supported/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
