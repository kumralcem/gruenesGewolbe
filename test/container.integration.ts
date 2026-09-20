import { spawnSync } from "node:child_process";
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

test("image application files are readable by the mapped worker UID", () => {
  const result = spawnSync(
    "podman",
    [
      "run",
      "--rm",
      "--network=none",
      "--read-only",
      "--cap-drop=ALL",
      "--userns=keep-id",
      `--user=${process.getuid?.() ?? 1000}:${process.getgid?.() ?? 1000}`,
      "--entrypoint",
      "node",
      "localhost/gg-pi-prototype",
      "-e",
      `const fs = require("node:fs");
     for (const path of ["/app/package.json", "/app/src/worker.ts"])
       fs.readFileSync(path);
     console.log("Worker code is readable");`,
    ],
    { encoding: "utf8", timeout: 90000 },
  );
  assert.equal(result.status, 0, result.stderr || String(result.error));
  assert.match(result.stdout, /Worker code is readable/);
});

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
      await readFile(join(result.outcome!.path!, "source.md"), "utf8"),
      source,
    );
  },
);

test(
  "an explicitly included linked article retains its URL and complete fetched text",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const input = "https://fixtures.example/post",
      article = "https://fixtures.example/article";
    const result = await runJob({
      vault,
      config,
      intent: "idea",
      input,
      fixturePage: (url) => ({
        title: "Fixture",
        text:
          url === input
            ? "A post linking an article."
            : "Article instructions: export, classify, verify, then save.",
      }),
      mockModel: async (body: any) => {
        const calls = body.messages
          .filter((m: any) => m.role === "assistant")
          .flatMap((m: any) => m.tool_calls ?? []);
        if (calls.length >= 3)
          return { role: "assistant", content: "Finished." };
        const name = calls.length < 2 ? "browse" : "capture";
        const args =
          calls.length === 0
            ? { url: input }
            : calls.length === 1
              ? { url: article, preserve: true }
              : {
                  title: "Linked instructions",
                  subvault: "Ideas",
                  summary: "Export, classify, verify, save.",
                  tags: ["instructions"],
                };
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
    const source = await readFile(
      join(result.outcome!.path!, "source.md"),
      "utf8",
    );
    assert.ok(source.includes(input) && source.includes(article));
    assert.match(source, /export, classify, verify, then save/);
  },
);

test(
  "capturing only an improved asset still retains the exact browser-selected original",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const sharp = (await import("sharp")).default;
    const image = async (width: number) =>
      sharp({
        create: { width, height: width, channels: 3, background: "#123456" },
      })
        .png()
        .toBuffer();
    const original = await image(100),
      better = await image(200);
    const result = await runJob({
      vault,
      config,
      intent: "art",
      input: "https://fixtures.example/art",
      browserCapture: {
        version: 1,
        intent: "art",
        url: "https://fixtures.example/art",
        title: "Selected original",
        text: "A square",
        capturedAt: "2026-09-11T10:00:00Z",
        image: {
          url: "https://fixtures.example/original",
          bytes: original.toString("base64"),
          mimeType: "image/png",
        },
      },
      fixtureFetch: async (url) => ({
        bytes: url.endsWith("original") ? original : better,
        type: "image/png",
        url,
      }),
      mockModel: async (body: any) => {
        const calls = body.messages
          .filter((m: any) => m.role === "assistant")
          .flatMap((m: any) => m.tool_calls ?? []);
        if (calls.length >= 2)
          return { role: "assistant", content: "Finished." };
        const name = calls.length < 1 ? "download_image" : "capture";
        const latest = body.messages
          .filter((m: any) => m.role === "tool")
          .at(-1);
        const args =
          calls.length < 1
            ? {
                url: "https://fixtures.example/better",
              }
            : {
                title: "Square",
                subvault: "Paintings",
                summary: "Square fixture",
                tags: [],
                selectedImage: "https://fixtures.example/original",
                assetIds: [JSON.parse(latest.content).assetId],
              };
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
    const [{ item, path }] = await vault.items();
    assert.equal(item.assets.length, 2);
    assert.deepEqual(await readFile(join(path, item.primary!)), better);
    assert.deepEqual(
      await readFile(
        join(path, item.assets.find((a) => a.file !== item.primary)!.file),
      ),
      original,
    );
  },
);

test(
  "a signed-in browser snapshot reaches real Pi without re-fetching the private page or image",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const fake = await fixtures();
    const fetched = await fake.fixtureFetch(
      "https://fixtures.example/image.png",
    );
    const result = await runJob({
      vault,
      config,
      intent: "art",
      input: "https://private.example/painting",
      browserCapture: {
        version: 1,
        intent: "art",
        url: "https://private.example/painting",
        title: "Painting from a signed-in page",
        text: "A painting",
        capturedAt: "2026-09-11T10:00:00Z",
        image: {
          url: "https://fixtures.example/image.png",
          bytes: fetched.bytes.toString("base64"),
          mimeType: "image/png",
        },
      },
      ...fake,
    });
    assert.equal(result.outcome?.status, "saved", JSON.stringify(result));
    assert.equal(
      result.gatewayEvents.some((e) =>
        ["/fetch", "/fixture-page"].includes(String(e.path)),
      ),
      false,
    );
    const record = await readFile(
      join(result.outcome!.path!, "record.md"),
      "utf8",
    );
    assert.match(record, /aliases:/);
    assert.match(record, /!\[/);
  },
);

test(
  "text-only capture into a requested nested folder is complete despite failed decorative images",
  { timeout: 60000 },
  async () => {
    const vault = await setup();
    const url = "https://fixtures.example/tips";
    const tool = (name: string, args: unknown) => ({
      role: "assistant",
      tool_calls: [
        {
          id: "call_" + name,
          type: "function",
          function: { name, arguments: JSON.stringify(args) },
        },
      ],
    });
    const replies = [
      tool("browse", { url }),
      tool("create_destination", { subvault: "Ideas/SoloDev" }),
      tool("capture", {
        kind: "idea",
        title: "Five tips",
        summary: "Five actionable instructions",
        subvault: "Ideas/SoloDev",
        tags: [],
        includeImages: false,
      }),
      { role: "assistant", content: "Saved." },
    ];
    let step = 0;
    const result = await runJob({
      vault,
      config,
      intent: "capture",
      input: url,
      browserCapture: {
        version: 2,
        intent: "capture",
        url,
        title: "Tips",
        text: "Five actionable instructions",
        capturedAt: "2026-09-20T00:00:00Z",
        instructions:
          "Save instructions without images and create SoloDev under Ideas.",
        images: [{ url: url + "/banner.png", error: "Failed to fetch" }],
      },
      mockModel: async () => replies[step++],
    });
    assert.equal(result.exitCode, 0, JSON.stringify(result));
    assert.equal(result.outcome?.status, "saved", JSON.stringify(result));
    assert.equal((await vault.items())[0].item.subvault, "Ideas/SoloDev");
  },
);
