import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  readFile,
  writeFile,
  unlink,
  symlink,
} from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { DEFAULT_CAPTURE_RULES } from "../src/capture-rules.ts";
import { createReceiver } from "../src/receiver.ts";

test("capture rules initialize once, detect external edits, reject stale saves and support undo", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-rules-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const original = await vault.captureRules();
  assert.equal(original.text, DEFAULT_CAPTURE_RULES);
  assert.match(original.text, /include every substantive item/);
  const saved = await vault.setCaptureRules(
    "List each tip and its example.",
    original.revision,
  );
  assert.equal(await readFile(join(root, "CAPTURE.md"), "utf8"), saved.text);
  await assert.rejects(
    vault.setCaptureRules("stale", original.revision),
    /changed since/,
  );
  await vault.undo();
  assert.equal((await vault.captureRules()).text, original.text);
  await writeFile(join(root, "CAPTURE.md"), "Manual preferences");
  assert.equal(
    (await (await Vault.open(root)).captureRules()).text,
    "Manual preferences",
  );
  await assert.rejects(
    vault.setCaptureRules("stale", original.revision),
    /changed since/,
  );
  await assert.rejects(
    vault.setCaptureRules("x".repeat(16001), original.revision),
    /16000/,
  );
  await unlink(join(root, "CAPTURE.md"));
  const outside = join(
    await mkdtemp(join(tmpdir(), "gg-outside-")),
    "rules.md",
  );
  await writeFile(outside, "outside");
  await symlink(outside, join(root, "CAPTURE.md"));
  await assert.rejects(vault.captureRules());
  await assert.rejects(vault.setCaptureRules("overwrite", original.revision));
  assert.equal(await readFile(outside, "utf8"), "outside");
});

test("paired extension settings read and save the same file, without unauthenticated writes", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-rules-http-")),
    ["Ideas"],
  );
  const receiver = await createReceiver({
    vault,
    port: 0,
    processCapture: async () => ({}),
  });
  try {
    const url = receiver.url + "/capture-rules";
    assert.equal((await fetch(url)).status, 401);
    assert.equal(
      (
        await fetch(url, {
          method: "POST",
          body: JSON.stringify({ text: "bad" }),
        })
      ).status,
      401,
    );
    const headers = {
      authorization: "Bearer " + receiver.token,
      "content-type": "application/json",
    };
    const initial = await (await fetch(url, { headers })).json();
    const response = await fetch(url, {
      method: "POST",
      headers,
      body: JSON.stringify({
        text: "Include every tip.",
        revision: initial.revision,
      }),
    });
    assert.equal(response.status, 200);
    assert.equal((await vault.captureRules()).text, "Include every tip.");
    const stale = await fetch(url, {
      method: "POST",
      headers,
      body: JSON.stringify({ text: "old tab", revision: initial.revision }),
    });
    assert.equal(stale.status, 400);
    assert.equal((await vault.captureRules()).text, "Include every tip.");
  } finally {
    await receiver.close();
  }
});
