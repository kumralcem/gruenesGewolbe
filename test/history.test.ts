import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { History } from "../src/history.ts";
test("history restores a batch and refuses to overwrite later edits", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-history-"));
  const history = new History(root);
  await writeFile(join(root, "a.md"), "original");
  await history.run(
    "edit",
    ["a.md"],
    () => writeFile(join(root, "a.md"), "second"),
    "batch",
  );
  await history.run(
    "edit",
    ["a.md"],
    () => writeFile(join(root, "a.md"), "third"),
    "batch",
  );
  await writeFile(join(root, "a.md"), "human");
  await assert.rejects(history.undo("batch"), /later edits/);
  assert.equal(await readFile(join(root, "a.md"), "utf8"), "human");
  await writeFile(join(root, "a.md"), "third");
  assert.equal((await history.undo("batch")).length, 2);
  assert.equal(await readFile(join(root, "a.md"), "utf8"), "original");
});
