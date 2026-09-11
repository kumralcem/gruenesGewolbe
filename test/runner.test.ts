import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { runJob } from "../src/runner.ts";

test(
  "cancelling before the runtime creates its container terminates the launcher and retries removal",
  { timeout: 15000 },
  async () => {
    const root = await mkdtemp(join(tmpdir(), "gg-startup-test-"));
    const pidFile = join(root, "pid");
    const cleanupFile = join(root, "cleanup");
    const runtime = `#!${process.execPath}\nconst fs=require('node:fs');\nif(process.argv[2]==='run'){fs.writeFileSync(${JSON.stringify(pidFile)},String(process.pid));setTimeout(()=>{},60000);}else{let running=false;try{process.kill(Number(fs.readFileSync(${JSON.stringify(pidFile)},'utf8')),0);running=true;}catch{}fs.appendFileSync(${JSON.stringify(cleanupFile)},running?'running\\n':'stopped\\n');process.exit(running?1:0);}`;
    await writeFile(join(root, "podman"), runtime, { mode: 0o700 });
    const oldPath = process.env.PATH;
    process.env.PATH = `${root}:${oldPath}`;
    try {
      const vault = await Vault.create(join(root, "vault"), ["Ideas"]);
      const result = await runJob({
        vault,
        intent: "ask",
        input: "query",
        config: { provider: "openai", model: "fixture", maxSeconds: 1 },
      });
      assert.match(result.outcome?.reason ?? "", /deadline/);
      assert.equal(await readFile(cleanupFile, "utf8"), "running\nstopped\n");
      const pid = Number(await readFile(pidFile, "utf8"));
      assert.throws(() => process.kill(pid, 0));
    } finally {
      process.env.PATH = oldPath;
    }
  },
);
