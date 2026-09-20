import { test } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Vault } from "../src/vault.ts";
import { createGateway } from "../src/gateway.ts";

function request(
  socketPath: string,
  token: string,
  path: string,
  body: unknown,
): Promise<{ status: number; data: any }> {
  return new Promise((resolve, reject) => {
    const req = http.request(
      {
        socketPath,
        path,
        method: "POST",
        headers: {
          authorization: `Bearer ${token}`,
          "content-type": "application/json",
        },
      },
      (res) => {
        let text = "";
        res.on("data", (c) => (text += c));
        res.on("end", () =>
          resolve({ status: res.statusCode!, data: JSON.parse(text) }),
        );
      },
    );
    req.on("error", reject);
    req.end(JSON.stringify(body));
  });
}
test("gateway authenticates requests, bounds model use, and blocks writes for ask", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-gate-test-")),
    ["Ideas"],
  );
  const gateway = await createGateway({
    vault,
    intent: "ask",
    input: "find notes",
    config: {
      provider: "openrouter",
      model: "future-model",
      apiKey: "secret-sentinel",
      maxRequests: 1,
    },
    mockModel: async () => ({ role: "assistant", content: "hello" }),
  });
  try {
    assert.equal(
      (await request(gateway.socket, "wrong", "/search", { query: "x" }))
        .status,
      401,
    );
    assert.equal(
      (await request(gateway.socket, gateway.token, "/save", {})).status,
      400,
    );
    assert.equal(
      (
        await request(gateway.socket, gateway.token, "/fetch", {
          url: "https://example.com",
        })
      ).status,
      400,
    );
    const response = await request(
      gateway.socket,
      gateway.token,
      "/model/chat/completions",
      { model: "future-model", messages: [], stream: false },
    );
    assert.equal(response.status, 200);
    assert.equal(JSON.stringify(response).includes("secret-sentinel"), false);
    assert.match(
      (
        await request(
          gateway.socket,
          gateway.token,
          "/model/chat/completions",
          { model: "future-model" },
        )
      ).data.error,
      /budget/,
    );
    assert.equal(
      (
        await request(gateway.socket, gateway.token, "/results", {
          items: [{ id: "invented", reason: "x" }],
        })
      ).status,
      400,
    );
    assert.deepEqual(await vault.items(), []);
  } finally {
    await gateway.close();
  }
});

test("gateway binds outcomes to one input and rejects concurrent terminal writes", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-once-test-")),
    ["Ideas"],
  );
  const sourceUrl = "https://example.org/source";
  const gateway = await createGateway({
    vault,
    intent: "idea",
    input: sourceUrl,
    config: { provider: "openai", model: "test" },
  });
  const draft = {
    kind: "idea",
    title: "Test",
    summary: "Actual steps",
    sourceText: "Source text",
    tags: [],
    subvault: "Ideas",
    sourceUrl,
  };
  try {
    assert.equal(
      (
        await request(gateway.socket, gateway.token, "/save", {
          ...draft,
          sourceUrl: "https://example.org/other",
        })
      ).status,
      400,
    );
    const results = await Promise.all([
      request(gateway.socket, gateway.token, "/save", draft),
      request(gateway.socket, gateway.token, "/queue", {
        sourceUrl,
        reason: "no-subvault",
        candidates: [],
      }),
    ]);
    assert.equal(results.filter((r) => r.status === 200).length, 1);
    assert.equal(gateway.outcomes.length, 1);
  } finally {
    await gateway.close();
  }
});

test("invalid budgets are rejected before opening a gateway", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-budget-test-")),
    ["Ideas"],
  );
  for (const maxSeconds of [0, NaN, Infinity, -1, 601])
    await assert.rejects(
      createGateway({
        vault,
        intent: "ask",
        input: "query",
        config: { provider: "openai", model: "future", maxSeconds },
      }),
      /maxSeconds/,
    );
});

test("model gateway disallows extra completions and provider-hosted tools", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-model-scope-test-")),
    ["Ideas"],
  );
  const gateway = await createGateway({
    vault,
    intent: "ask",
    input: "find notes",
    config: { provider: "openai", model: "future" },
    mockModel: async () => ({ role: "assistant", content: "ok" }),
  });
  try {
    for (const body of [
      { n: 2 },
      { plugins: [{ id: "web" }] },
      { models: ["different-model"] },
      { provider: { order: ["unconfigured"] } },
      { tools: [{ type: "web_search" }] },
      { tools: [{ type: "code_interpreter" }] },
      { background: true },
      { previous_response_id: "other-job" },
    ]) {
      const response = await request(
        gateway.socket,
        gateway.token,
        "/model/chat/completions",
        { model: "future", ...body },
      );
      assert.equal(response.status, 400);
    }
    assert.equal(
      (
        await request(
          gateway.socket,
          gateway.token,
          "/model/chat/completions",
          {
            model: "future",
            n: 1,
            tools: [
              {
                type: "function",
                function: { name: "search", parameters: { type: "object" } },
              },
            ],
          },
        )
      ).status,
      200,
    );
  } finally {
    await gateway.close();
  }
});

test("browser art saves are bound to the user's selected URL and original bytes", async () => {
  const sharp = (await import("sharp")).default;
  const { createHash } = await import("node:crypto");
  const make = async (color: string) => {
    const bytes = await sharp({
      create: { width: 100, height: 100, channels: 3, background: color },
    })
      .png()
      .toBuffer();
    return {
      bytes: bytes.toString("base64"),
      width: 100,
      height: 100,
      visualHash: createHash("sha256")
        .update(await sharp(bytes).resize(32, 32).raw().toBuffer())
        .digest("hex"),
    };
  };
  const original = await make("#123456"),
    unrelated = await make("#654321");
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-selected-test-")),
    ["Paintings"],
  );
  const url = "https://private.example/post",
    imageUrl = url + "/image.png";
  const gateway = await createGateway({
    vault,
    intent: "art",
    input: url,
    config: { provider: "openai", model: "test" },
    browserCapture: {
      version: 1,
      intent: "art",
      url,
      title: "Selected art",
      capturedAt: "2026-09-11T10:00:00Z",
      text: "",
      image: { url: imageUrl, bytes: original.bytes, mimeType: "image/png" },
    },
  });
  const draft = {
    kind: "art",
    title: "Square",
    subvault: "Paintings",
    summary: "Selected square",
    tags: [],
    sourceUrl: url,
    selectedImage: imageUrl,
    assets: [original],
  };
  try {
    for (const bad of [
      { ...draft, selectedImage: url + "/other" },
      { ...draft, assets: [unrelated] },
    ]) {
      const result = await request(gateway.socket, gateway.token, "/save", bad);
      assert.equal(result.status, 400);
      assert.match(result.data.error, /exact browser-selected image/);
    }
    assert.equal((await vault.items()).length, 0);
    assert.equal(
      (await request(gateway.socket, gateway.token, "/save", draft)).status,
      200,
    );
  } finally {
    await gateway.close();
  }
});

test("browser multi-record plans retain partial saves and retries do not duplicate records", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-multiple-test-")),
    ["Ideas"],
  );
  const input = "https://private.example/article";
  const options = {
    vault,
    intent: "capture" as const,
    input,
    config: { provider: "openai" as const, model: "fixture" },
    browserCapture: {
      version: 2 as const,
      intent: "capture" as const,
      url: input,
      title: "Two ideas",
      capturedAt: "2026-09-15T00:00:00Z",
      text: "Original source",
      contextText: "Useful reply",
      instructions: "Create two records",
      images: [],
    },
  };
  const save = {
    sourceUrl: input,
    kind: "idea",
    subvault: "unknown",
    title: "First",
    summary: "Useful summary",
    tags: [],
    sourceText: "invented source",
    captureKey: "first",
  };
  const first = await createGateway(options);
  try {
    assert.equal(
      (
        await request(first.socket, first.token, "/plan", {
          keys: ["first", "second"],
        })
      ).status,
      200,
    );
    assert.equal(
      (await request(first.socket, first.token, "/save", save)).status,
      200,
    );
    assert.equal(first.complete, false);
  } finally {
    await first.close();
  }
  const newer = await createGateway({
    ...options,
    browserCapture: {
      ...options.browserCapture,
      capturedAt: "2026-09-16T01:00:00Z",
    },
  });
  try {
    await request(newer.socket, newer.token, "/plan", {
      keys: ["first", "second"],
    });
    assert.equal(
      (
        await request(newer.socket, newer.token, "/save", {
          ...save,
          summary: "Newer content must survive",
        })
      ).status,
      200,
    );
  } finally {
    await newer.close();
  }
  const retry = await createGateway(options);
  try {
    const context = await request(
      retry.socket,
      retry.token,
      "/capture-context",
      {},
    );
    assert.deepEqual(context.data.completedKeys, ["first"]);
    assert.deepEqual(context.data.pendingKeys, ["second"]);
    assert.equal(
      (await request(retry.socket, retry.token, "/plan", { keys: ["first"] }))
        .status,
      400,
    );
    assert.equal(retry.complete, false);
    await request(retry.socket, retry.token, "/plan", {
      keys: ["first", "second"],
    });
    assert.equal(
      (await request(retry.socket, retry.token, "/save", save)).data.status,
      "existing",
    );
    assert.equal(
      (
        await request(retry.socket, retry.token, "/save", {
          ...save,
          title: "Second",
          captureKey: "second",
          includeContext: true,
        })
      ).status,
      200,
    );
    assert.equal(retry.complete, true);
    assert.equal(retry.batchId, first.batchId);
    assert.equal(
      (await vault.items()).find((v) => v.item.captureKey === "first")?.item
        .summary,
      "Newer content must survive",
    );
    assert.equal(
      (await request(retry.socket, retry.token, "/confirm", { id: "made-up" }))
        .status,
      404,
    );
    const items = await vault.items();
    assert.equal(items.length, 2);
    assert.ok(items.every((v) => v.item.subvault === "Inbox"));
    assert.equal(
      (
        await vault.read(
          items.find((v) => v.item.captureKey === "first")!.item.id,
        )
      ).sourceText,
      "Original source",
    );
    assert.match(
      (
        await vault.read(
          items.find((v) => v.item.captureKey === "second")!.item.id,
        )
      ).sourceText,
      /Useful reply/,
    );
  } finally {
    await retry.close();
  }
});

test("capture can create a requested destination and ignore irrelevant images without enabling general management", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-capture-folder-")),
    ["Ideas"],
  );
  const url = "https://example.com/tips";
  const options = {
    vault,
    intent: "capture" as const,
    input: url,
    config: { provider: "openai" as const, model: "fixture" },
    browserCapture: {
      version: 2 as const,
      intent: "capture" as const,
      url,
      title: "Tips",
      text: "Five actionable steps",
      capturedAt: "2026-09-20T00:00:00Z",
      images: [{ url: url + "/decoration.png", error: "Failed to fetch" }],
    },
  };
  const denied = await createGateway(options);
  try {
    assert.notEqual(
      (
        await request(denied.socket, denied.token, "/create-destination", {
          subvault: "Ideas/SoloDev",
        })
      ).status,
      200,
    );
  } finally {
    await denied.close();
  }
  const gateway = await createGateway({
    ...options,
    instructions:
      "Save as instructions without images; create SoloDev under Ideas.",
  });
  try {
    assert.equal(
      (
        await request(gateway.socket, gateway.token, "/create-destination", {
          subvault: "Ideas/SoloDev",
        })
      ).status,
      200,
    );
    assert.equal(
      (
        await request(gateway.socket, gateway.token, "/create-destination", {
          subvault: "Ideas/SoloDev",
        })
      ).data.existing,
      true,
    );
    assert.notEqual(
      (
        await request(gateway.socket, gateway.token, "/manage", {
          action: "rename-subvault",
          from: "Ideas",
          subvault: "Other",
        })
      ).status,
      200,
    );
    const draft = {
      kind: "idea",
      title: "Five tips",
      sourceUrl: url,
      summary: "Five actionable steps",
      subvault: "Ideas/SoloDev",
      tags: [],
      includeImages: false,
    };
    assert.notEqual(
      (
        await request(gateway.socket, gateway.token, "/save", {
          ...draft,
          kind: "art",
        })
      ).status,
      200,
    );
    const saved = await request(gateway.socket, gateway.token, "/save", draft);
    assert.equal(saved.status, 200, JSON.stringify(saved.data));
    assert.equal(saved.data.status, "saved");
    assert.equal((await vault.items())[0].item.subvault, "Ideas/SoloDev");
    assert.equal((await vault.items())[0].item.missingMedia?.length ?? 0, 0);
  } finally {
    await gateway.close();
  }
});
