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

test(
  "long browser transcripts fit the normal model bound and normalized post URLs stay on the snapshot",
  { timeout: 60000 },
  async () => {
    const sharp = (await import("sharp")).default;
    const bytes = (
      await sharp({
        create: { width: 200, height: 200, channels: 3, background: "#abcdef" },
      })
        .png()
        .toBuffer()
    ).toString("base64");
    for (const video of [true, false]) {
      const vault = await Vault.create(
        await mkdtemp(join(tmpdir(), "gg-context-size-")),
        ["Ideas"],
      );
      const url = video
        ? "https://www.youtube.com/watch?v=abcdefghijk"
        : "https://x.com/example/status/12345?s=20";
      const requested = video ? url : "https://x.com/example/status/12345";
      const steps = [
        call("browse", { url: requested }),
        video
          ? call("read_snapshot", { section: "transcript", offset: 12000 })
          : call("fetch_text", { url: requested }),
        call("capture", {
          kind: "idea",
          title: "Instructions",
          summary: "Detailed instructions",
          subvault: "Ideas",
          includeImages: false,
          tags: [],
        }),
        { role: "assistant", content: "Saved" },
      ];
      let step = 0;
      const result = await worker({
        vault,
        config: { provider: "openai", model: "fixture", maxSeconds: 20 },
        intent: "capture",
        input: url,
        browserCapture: {
          version: 2,
          intent: "capture",
          url,
          title: "Source",
          text: "page text ".repeat(900),
          html: "<div>page structure</div>".repeat(13000),
          transcript: video ? "transcript sentence. ".repeat(1250) : undefined,
          capturedAt: "2026-09-20T00:00:00Z",
          instructions: "Save the instructions without images.",
          images: [
            { url: url + "#image1", bytes, mimeType: "image/png" },
            { url: url + "#image2", bytes, mimeType: "image/png" },
          ],
        },
        fixturePage: () => {
          throw Error("The supplied source must not be re-fetched");
        },
        mockModel: async (body) => {
          if (step === 1)
            assert.match(body.messages.at(-1).content, /"fromBrowser":true/);
          return steps[step++];
        },
      });
      assert.equal(result.outcomes[0].status, "saved", result.output);
    }
  },
);

// Opt-in local replay keeps private browser snapshots out of the repository.
test(
  "stored browser snapshots replay within the model bound",
  { skip: !process.env.GG_REPLAY_INPUTS, timeout: 60000 },
  async () => {
    const { readFile } = await import("node:fs/promises");
    for (const path of JSON.parse(process.env.GG_REPLAY_INPUTS ?? "[]")) {
      const snapshot = JSON.parse(await readFile(path, "utf8"));
      const vault = await Vault.create(
        await mkdtemp(join(tmpdir(), "gg-private-replay-")),
        ["Ideas"],
      );
      const url = snapshot.url.includes("x.com/")
        ? snapshot.url.split("?")[0]
        : snapshot.url;
      const steps = [call("browse", { url })];
      for (
        let offset = 12000;
        offset < (snapshot.transcript?.length ?? 0);
        offset += 12000
      )
        steps.push(call("read_snapshot", { section: "transcript", offset }));
      if (!snapshot.transcript) steps.push(call("fetch_text", { url }));
      steps.push(
        call("capture", {
          kind: "idea",
          title: "Replay",
          summary: "Fixture replay only",
          subvault: "Ideas",
          tags: [],
          includeImages: false,
        }),
      );
      let step = 0;
      const result = await worker({
        vault,
        intent: "capture",
        input: snapshot.url,
        browserCapture: snapshot,
        config: { provider: "openai", model: "fixture", maxSeconds: 20 },
        fixturePage: () => {
          throw Error("Unexpected public refetch");
        },
        mockModel: async () =>
          steps[step++] ?? { role: "assistant", content: "Saved" },
      });
      assert.equal(result.outcomes[0].status, "saved", result.output);
    }
  },
);

test(
  "real Pi imports a local image through the gateway and preserves exact original bytes",
  { timeout: 30000 },
  async () => {
    const sharp = (await import("sharp")).default;
    const { writeFile, readFile, rm } = await import("node:fs/promises");
    const { imageCapture } = await import("../src/image-import.ts");
    const root = await mkdtemp(join(tmpdir(), "gg-local-worker-"));
    try {
      const file = join(root, "painting.png");
      const bytes = await sharp({
        create: { width: 20, height: 20, channels: 3, background: "green" },
      })
        .png()
        .toBuffer();
      await writeFile(file, bytes);
      const capture = await imageCapture(file, "Save this in Paintings");
      const vault = await Vault.create(join(root, "vault"), ["Paintings"]);
      let step = 0;
      let assetId = "";
      const source = "https://museum.example/green";
      const result = await worker({
        vault,
        config: {
          provider: "openai",
          model: "fixture",
          maxSeconds: 25,
          maxRequests: 6,
        },
        intent: "capture",
        input: capture.url,
        browserCapture: capture,
        fixtureFetch: async (url) => ({
          url,
          type: "text/html",
          bytes: Buffer.from(
            "<p>Green painting. Artist Example. Created 1918.</p>",
          ),
        }),
        mockModel: async (body) => {
          if (step++ === 0) return call("browse", { url: capture.url });
          if (step === 2) {
            const view = JSON.parse(
              body.messages.filter((m: any) => m.role === "tool").at(-1)
                .content,
            );
            assert.equal(view.previewAssets.length, 1);
            assetId = view.previewAssets[0].assetId;
            return call("fetch_text", { url: source });
          }
          if (step === 3) {
            return call("capture", {
              kind: "art",
              title: "Green painting",
              summary: "A green painting",
              subvault: "Paintings",
              tags: [],
              assetIds: [assetId],
              attribution: {
                year: {
                  value: "1918",
                  status: "source-supported",
                  sourceUrl: source,
                  quote: "Created 1918",
                },
                creator: {
                  value: "Artist Example",
                  status: "source-supported",
                  sourceUrl: source,
                  quote: "Artist Example",
                },
              },
            });
          }
          return { role: "assistant", content: "Saved." };
        },
      });
      assert.equal(result.outcomes[0]?.status, "saved");
      const [record] = await vault.sourceRecords(capture.url);
      assert.equal(record.item.year, "1918");
      assert.equal(record.item.creator, "Artist Example");
      assert.equal(record.item.attribution?.title?.status, "uncertain");
      assert.deepEqual(
        await readFile(join(record.path, record.item.assets[0].file)),
        bytes,
      );
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  },
);

test("a completed capture needs no additional model request to announce success", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-final-save-")),
    ["Ideas"],
  );
  let requests = 0;
  const result = await worker({
    vault,
    intent: "capture",
    input: "https://example.com/one",
    config: { provider: "openai", model: "fixture", maxRequests: 1 },
    browserCapture: {
      version: 2,
      intent: "capture",
      url: "https://example.com/one",
      title: "One",
      text: "Useful steps",
      capturedAt: "2026-09-20T00:00:00Z",
      images: [],
    },
    mockModel: async () => {
      assert.equal(
        ++requests,
        1,
        "No paid model request after the successful final save",
      );
      return call("capture", {
        kind: "idea",
        title: "One",
        summary: "Useful steps",
        subvault: "Ideas",
        tags: [],
        includeImages: false,
      });
    },
  });
  assert.equal(requests, 1);
  assert.equal(result.complete, true);
});

test("local research closes with space to save instead of overflowing on the next lookup", async () => {
  const sharp = (await import("sharp")).default;
  const { writeFile } = await import("node:fs/promises");
  const { imageCapture } = await import("../src/image-import.ts");
  const root = await mkdtemp(join(tmpdir(), "gg-research-budget-"));
  const file = join(root, "painting.png");
  await writeFile(
    file,
    await sharp({
      create: { width: 100, height: 100, channels: 3, background: "green" },
    })
      .png()
      .toBuffer(),
  );
  const capture = await imageCapture(file);
  const vault = await Vault.create(join(root, "vault"), ["Art"]);
  let step = 0,
    assetId = "";
  const result = await worker({
    vault,
    intent: "capture",
    input: capture.url,
    browserCapture: capture,
    config: { provider: "openai", model: "fixture", maxRequests: 3 },
    fixturePage: () => ({
      text: "Research evidence. ".repeat(1550),
      images: [],
    }),
    mockModel: async (body) => {
      if (step++ === 0) return call("browse", { url: capture.url });
      if (step === 2) {
        assetId = JSON.parse(
          body.messages.filter((m: any) => m.role === "tool").at(-1).content,
        ).previewAssets[0].assetId;
        return call("browse", { url: "https://example.com/research" });
      }
      assert.match(body.messages[0].content, /research budget is now closed/);
      return call("capture", {
        kind: "art",
        title: "Unidentified painting",
        subvault: "Art",
        summary: "Green image",
        tags: [],
        assetIds: [assetId],
      });
    },
  });
  assert.equal(result.complete, true);
  assert.equal(step, 3);
});
