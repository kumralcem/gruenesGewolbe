// Exercises real Pi and the native gateway protocol without a container.
// This is deliberately NOT an isolation test; test:container remains required on the deployment host.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import type { Duplex } from "node:stream";
import { Channel } from "../src/ipc.ts";
import { createGateway, type GatewayOptions } from "../src/gateway.ts";
import { Vault } from "../src/vault.ts";
async function worker(options: GatewayOptions) {
  const work = await mkdtemp(join(tmpdir(), "gg-worker-fixture-"));
  const gateway = await createGateway(options);
  const child = spawn(
    process.execPath,
    [
      "--import",
      resolve("node_modules/tsx/dist/loader.mjs"),
      resolve("src/worker.ts"),
    ],
    {
      stdio: ["pipe", "pipe", "pipe", "pipe"],
      env: {
        PATH: process.env.PATH,
        HOME: work,
        GG_FIXTURE_WORK_DIR: work,
        PI_OFFLINE: "1",
      },
    },
  );
  const channel = new Channel(child.stdio[3] as Duplex, gateway.socket);
  let output = "",
    errors = "";
  child.stdout.on("data", (b) => (output += b));
  child.stderr.on("data", (b) => (errors += b));
  const timer = setTimeout(() => child.kill("SIGKILL"), 20000);
  child.stdin.end(
    JSON.stringify({
      intent: options.intent,
      input: options.input,
      token: gateway.token,
      config: options.config,
      areas: options.vault.areas,
      fixtures: true,
      browserCapture: !!options.browserCapture,
    }),
  );
  try {
    const exit = await new Promise((r) => child.once("exit", r));
    assert.equal(exit, 0, errors + output);
    return {
      outcomes: gateway.outcomes,
      management: gateway.management,
      complete: gateway.complete,
      output,
    };
  } finally {
    clearTimeout(timer);
    child.kill();
    channel.close();
    await gateway.close();
  }
}
const call = (name: string, args: unknown) => ({
  role: "assistant",
  tool_calls: [
    {
      id: "call_" + name,
      type: "function",
      function: { name, arguments: JSON.stringify(args) },
    },
  ],
});
test(
  "real Pi captures private snapshots with split records, then manages the vault with confirmation outside the model",
  { timeout: 60000 },
  async () => {
    const vault = await Vault.create(
      await mkdtemp(join(tmpdir(), "gg-pi-fixture-")),
      ["Ideas"],
    );
    await mkdir(join(vault.root, "subvaults", "Ideas", "Historic"));
    const sourceUrl = "https://private.example/post";
    const config = {
      provider: "openai" as const,
      model: "fixture",
      maxSeconds: 25,
      maxRequests: 10,
    };
    const steps = [
      call("browse", { url: sourceUrl }),
      call("plan_capture", { keys: ["first", "second"] }),
      call("capture", {
        captureKey: "first",
        kind: "idea",
        title: "First idea",
        summary: "First usable idea",
        subvault: "Ideas/Historic",
        tags: [],
      }),
      call("capture", {
        captureKey: "second",
        kind: "idea",
        title: "Second idea",
        summary: "Second usable idea",
        subvault: "Ideas",
        tags: [],
      }),
      { role: "assistant", content: "Saved both records." },
    ];
    let at = 0;
    const result = await worker({
      vault,
      config,
      intent: "capture",
      input: sourceUrl,
      browserCapture: {
        version: 2,
        intent: "capture",
        url: sourceUrl,
        title: "Private ideas",
        text: "Only available inside this snapshot",
        capturedAt: "2026-09-16T00:00:00Z",
        instructions: "Create two records",
        images: [],
      },
      mockModel: async (body) => {
        assert.match(body.messages[0].content, /Ideas\/Historic/);
        return steps[at++];
      },
    });
    assert.equal(result.complete, true, result.output);
    assert.equal((await vault.items()).length, 2);
    assert.equal(
      (await vault.items()).find((v) => v.item.title === "First idea")?.item
        .subvault,
      "Ideas/Historic",
    );
    const id = (await vault.items())[0].item.id;
    const manageSteps = [
      call("vault_catalog", {}),
      call("manage_vault", { action: "delete", id }),
      { role: "assistant", content: "Confirm the deletion preview." },
    ];
    at = 0;
    const managed = await worker({
      vault,
      config,
      intent: "manage",
      input: "Delete the first idea",
      mockModel: async () => manageSteps[at++],
    });
    assert.equal((await vault.items()).length, 2);
    assert.equal(managed.management.length, 1);
    const preview = managed.management[0] as { proposalId: string };
    assert.ok(preview.proposalId);
    await vault.confirm(preview.proposalId);
    assert.equal((await vault.items()).length, 1);
  },
);

test(
  "real Pi preserves several supplied images and flags undecodable media without losing content",
  { timeout: 30000 },
  async () => {
    const sharp = (await import("sharp")).default;
    const { readFile } = await import("node:fs/promises");
    const vault = await Vault.create(
      await mkdtemp(join(tmpdir(), "gg-image-worker-")),
      ["Paintings"],
    );
    const url = "https://private.example/gallery";
    const originals = await Promise.all(
      ["#aabbcc", "#ccbbaa"].map((background) =>
        sharp({ create: { width: 100, height: 100, channels: 3, background } })
          .png()
          .toBuffer(),
      ),
    );
    const images = originals.map((bytes, i) => ({
      url: url + "/" + i,
      bytes: bytes.toString("base64"),
      mimeType: "image/png",
    }));
    images.push({
      url: url + "/corrupt",
      bytes: Buffer.from("not an image").toString("base64"),
      mimeType: "image/png",
    });
    let step = 0,
      assetIds: string[] = [];
    const result = await worker({
      vault,
      intent: "capture",
      input: url,
      config: { provider: "openai", model: "fixture", maxSeconds: 20 },
      browserCapture: {
        version: 2,
        intent: "capture",
        url,
        title: "Gallery",
        text: "Image captions",
        capturedAt: "2026-09-16T02:00:00Z",
        images,
      },
      mockModel: async (body) => {
        switch (step++) {
          case 0:
            return call("browse", { url });
          case 1:
            assetIds = JSON.parse(
              body.messages.filter((m: any) => m.role === "tool").at(-1)
                .content,
            ).previewAssets.map((p: any) => p.assetId);
            assert.equal(assetIds.length, 2);
            return call("inspect_images", { urls: [url + "/corrupt"] });
          case 2:
            return call("capture", {
              kind: "art",
              title: "Gallery",
              summary: "Two paintings with captions",
              subvault: "Paintings",
              tags: [],
              assetIds,
            });
          default:
            return {
              role: "assistant",
              content: "Saved available images and reported the missing image.",
            };
        }
      },
    });
    assert.equal(result.complete, true, result.output);
    assert.equal(result.outcomes[0].status, "partial");
    const [{ item, path }] = await vault.items();
    assert.equal(item.assets.length, 2);
    assert.equal(item.missingMedia?.length, 1);
    const stored = await Promise.all(
      item.assets.map((a) => readFile(join(path, a.file))),
    );
    for (const original of originals)
      assert.ok(stored.some((b) => b.equals(original)));
    const record = await readFile(join(path, "record.md"), "utf8");
    assert.equal((record.match(/!\[Gallery\]/g) ?? []).length, 2);
    assert.match(record, /Capture notes/);
  },
);

test(
  "Pi saves text instructions into a requested new nested destination despite irrelevant failed images",
  { timeout: 30000 },
  async () => {
    const vault = await Vault.create(
      await mkdtemp(join(tmpdir(), "gg-text-instructions-")),
      ["Ideas"],
    );
    const rules = await vault.captureRules();
    await vault.setCaptureRules(
      "CUSTOM_DEFAULT: Explain all five tips and give actionable examples.",
      rules.revision,
    );
    const url = "https://example.com/five-tips";
    const steps = [
      call("browse", { url }),
      call("create_destination", { subvault: "Ideas/SoloDev" }),
      call("capture", {
        kind: "idea",
        title: "Five tips",
        summary:
          "1. Research users. 2. Improve onboarding. 3. Measure retention. 4. Test acquisition. 5. Iterate.",
        subvault: "Ideas/SoloDev",
        includeImages: false,
        tags: [],
      }),
      { role: "assistant", content: "Saved the instruction set." },
    ];
    let step = 0;
    const result = await worker({
      vault,
      intent: "capture",
      input: url,
      config: { provider: "openai", model: "fixture", maxSeconds: 20 },
      browserCapture: {
        version: 2,
        intent: "capture",
        url,
        title: "Five tips",
        text: "Five actionable tips",
        capturedAt: "2026-09-20T00:00:00Z",
        instructions:
          "Save as an instruction set; create SoloDev under Ideas. Images are irrelevant.",
        images: [{ url: url + "/banner", error: "Failed to fetch" }],
      },
      mockModel: async (body) => {
        const prompt = body.messages[0].content;
        assert.match(prompt, /CUSTOM_DEFAULT: Explain all five tips/);
        assert.match(prompt, /individual capture take precedence/);
        assert.match(prompt, /Save as an instruction set/);
        return steps[step++];
      },
    });
    assert.equal(result.outcomes[0].status, "saved", result.output);
    const item = (await vault.items())[0].item;
    assert.equal(item.subvault, "Ideas/SoloDev");
    assert.equal(item.assets.length, 0);
    assert.equal(item.missingMedia?.length ?? 0, 0);
  },
);
