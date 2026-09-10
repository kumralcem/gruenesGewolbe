import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { runJob } from "../src/runner.ts";
import { fixtures } from "../src/fixtures.ts";
const config = {
  provider: "openai" as const,
  model: "fixture-model",
  api: "openai-completions" as const,
  apiKey: "REAL_KEY_SENTINEL",
  maxSeconds: 40,
  maxRequests: 8,
};
async function setup() {
  return Vault.create(await mkdtemp(join(tmpdir(), "gg-container-test-")), [
    "Paintings",
    "Ideas",
  ]);
}

test(
  "worker has a working broker/browser, but no host files, real credentials or direct network",
  { timeout: 60000 },
  async () => {
    const r = await runJob({
      vault: await setup(),
      config,
      intent: "probe",
      input: "probe",
    });
    const checks = r.events.find((e) => e.type === "probe")?.checks;
    assert.equal(r.exitCode, 0, r.stderr);
    assert.ok(checks);
    assert.equal(checks.uid, process.getuid?.());
    assert.equal(checks.bridge, true);
    for (const field of [
      "realKeysInEnv",
      "/home/cem",
      "/root/.ssh",
      "/vault",
      "/var/run/docker.sock",
    ])
      assert.equal(checks[field], false, field);
    for (const field of [
      "privateTarget",
      "directNetworkBlocked",
      "hostWriteDenied",
      "scratchWritable",
    ])
      assert.equal(checks[field], true, field);
    assert.equal(checks.browser, "Browser sandbox works");
    assert.equal(JSON.stringify(r).includes("REAL_KEY_SENTINEL"), false);
  },
);
test(
  "real Pi processes failed, ambiguous, successful and repeated fixture entries independently",
  { timeout: 180000 },
  async () => {
    const vault = await setup();
    const fake = await fixtures();
    const states = [];
    for (const path of ["blocked", "ambiguous", "art", "art"]) {
      const r = await runJob({
        vault,
        config,
        intent: "art",
        input: `https://fixtures.example/${path}`,
        ...fake,
      });
      assert.equal(r.exitCode, 0, JSON.stringify(r));
      states.push(r.outcome?.status);
    }
    assert.deepEqual(states, ["skipped", "queued", "saved", "existing"]);
    assert.equal((await vault.items()).length, 1);
    const [{ item, path }] = await vault.items();
    assert.ok((await readFile(join(path, item.primary!))).length > 0);
  },
);
test(
  "OpenRouter protocol and missing-transcript policy run through real Pi",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const r = await runJob({
      vault,
      config: { ...config, provider: "openrouter" },
      intent: "idea",
      input: "https://fixtures.example/video",
      ...(await fixtures()),
    });
    assert.equal(r.exitCode, 0, JSON.stringify(r));
    assert.equal(r.outcome?.status, "skipped");
    assert.equal((await vault.items()).length, 0);
  },
);
test(
  "ask resolves verified file paths and leaves the stored record unchanged",
  { timeout: 100000 },
  async () => {
    const vault = await setup();
    const saved = await vault.save({
      kind: "idea",
      title: "Support inbox triage",
      summary:
        "Classify customer messages by urgency, draft replies, and require human review before sending.",
      tags: ["email", "support"],
      subvault: "Ideas",
      sourceUrl: "https://example.org/support",
      sourceText:
        "An email assistant tags requests, drafts a response, and escalates sensitive cases.",
    });
    const before = await readFile(join(saved.path!, "record.md"), "utf8");
    const r = await runJob({
      vault,
      config,
      intent: "ask",
      input: "managing customers using email agents",
      ...(await fixtures()),
    });
    assert.equal(r.exitCode, 0, JSON.stringify(r));
    assert.equal(r.answers.length, 1);
    assert.equal((r.answers[0] as any).path, saved.path);
    assert.equal(
      await readFile(join(saved.path!, "record.md"), "utf8"),
      before,
    );
    const no = await runJob({
      vault,
      config,
      intent: "ask",
      input: "no-match",
      ...(await fixtures()),
    });
    assert.deepEqual(no.answers, []);
  },
);

test(
  "a hung model job is cancelled at its deadline and the next entry can run",
  { timeout: 45000 },
  async () => {
    const vault = await setup();
    const start = Date.now();
    const timed = await runJob({
      vault,
      config: { ...config, maxSeconds: 3 },
      intent: "art",
      input: "https://fixtures.example/art",
      ...(await fixtures()),
      mockModel: async () => new Promise(() => {}),
    });
    assert.equal(timed.outcome?.status, "failed");
    assert.match(timed.outcome?.reason ?? "", /deadline/);
    assert.ok(Date.now() - start < 20000);
    const next = await runJob({
      vault,
      config,
      intent: "art",
      input: "https://fixtures.example/art",
      ...(await fixtures()),
    });
    assert.equal(next.outcome?.status, "saved");
  },
);

test(
  "idea capture preserves fetched source independently of the generated summary or excerpt",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const source =
      "First export your inbox. Then classify messages. Keep a rollback copy. SOURCE-END-471";
    const result = await runJob({
      vault,
      config,
      intent: "idea",
      input: "https://fixtures.example/instructions",
      focus: "classification",
      fixturePage: () => ({ title: "Instructions", text: source }),
      mockModel: async (body: any) => {
        const calls = body.messages
          .filter((m: any) => m.role === "assistant")
          .flatMap((m: any) => m.tool_calls ?? []);
        if (calls.length >= 2)
          return { role: "assistant", content: "Finished." };
        const name = calls.length ? "capture" : "browse";
        const args = calls.length
          ? {
              title: "Inbox instructions",
              subvault: "Ideas",
              summary: "Classify messages after exporting.",
              sourceText: "This agent excerpt must not replace the source.",
              tags: ["inbox"],
            }
          : { url: "https://fixtures.example/instructions" };
        return {
          role: "assistant",
          content: null,
          tool_calls: [
            {
              id: `call_${calls.length}`,
              type: "function",
              function: { name, arguments: JSON.stringify(args) },
            },
          ],
        };
      },
    });
    assert.equal(result.outcome?.status, "saved", JSON.stringify(result));
    assert.equal(
      await readFile(join(result.outcome!.path!, "source.txt"), "utf8"),
      source,
    );
  },
);
