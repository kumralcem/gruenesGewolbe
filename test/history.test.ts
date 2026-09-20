import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readdir, writeFile, readFile } from "node:fs/promises";
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

test("undo preserves a pre-existing empty directory when removing its first file", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-history-directory-"));
  const history = new History(root);
  await mkdir(join(root, "existing"));
  const change = await history.run("add", ["existing"], () =>
    writeFile(join(root, "existing", "note.md"), "new"),
  );
  await history.undo(change.operationId);
  assert.deepEqual(await readdir(join(root, "existing")), []);
});

test("legacy folder undo refuses to claim success without directory history", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-history-legacy-"));
  const history = new History(root);
  const change = await history.run("create-subvault", ["legacy"], () =>
    mkdir(join(root, "legacy")),
  );
  const path = join(root, ".gg-history", change.operationId + ".json");
  const journal = JSON.parse(await readFile(path, "utf8"));
  journal.after = {};
  await writeFile(path, JSON.stringify(journal));
  await assert.rejects(
    history.undo(change.operationId),
    /Older folder history/,
  );
  assert.deepEqual(await readdir(join(root, "legacy")), []);
  assert.equal((await history.list())[0].undone, undefined);
});
