import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { spawn } from "node:child_process";
import { fileLock } from "../src/state.ts";
test("competing processes reclaim a dead owner's lock without entering together", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-lock-test-"));
  const lock = join(root, "lock"),
    log = join(root, "events");
  await writeFile(lock, "2147483647");
  const script = `import {fileLock} from ${JSON.stringify(resolve("src/state.ts"))};import {appendFile} from 'node:fs/promises';await fileLock(${JSON.stringify(lock)},async()=>{await appendFile(${JSON.stringify(log)},'start '+process.pid+'\\n');await new Promise(r=>setTimeout(r,40));await appendFile(${JSON.stringify(log)},'end '+process.pid+'\\n');},10000);`;
  await Promise.all(
    Array.from(
      { length: 6 },
      () =>
        new Promise<void>((resolve, reject) => {
          const p = spawn(
            process.execPath,
            ["--import", "tsx", "--input-type=module", "-e", script],
            { stdio: ["ignore", "ignore", "pipe"] },
          );
          let errors = "";
          p.stderr.on("data", (c) => (errors += c));
          p.on("error", reject);
          p.on("exit", (code) =>
            code === 0 ? resolve() : reject(Error(errors)),
          );
        }),
    ),
  );
  const lines = (await readFile(log, "utf8")).trim().split("\n");
  assert.equal(lines.length, 12);
  for (let i = 0; i < lines.length; i += 2) {
    assert.match(lines[i], /^start /);
    assert.equal(lines[i + 1], lines[i].replace("start ", "end "));
  }
});
test("waiting for a job lock respects cancellation", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-cancel-lock-"));
  const lock = join(root, "lock");
  await writeFile(lock, String(process.pid));
  const abort = new AbortController();
  const waiting = fileLock(
    lock,
    async () => {
      throw Error("must not enter");
    },
    10000,
    abort.signal,
  );
  abort.abort();
  await assert.rejects(waiting, /abort/i);
});
