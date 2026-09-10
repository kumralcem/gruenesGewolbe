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
