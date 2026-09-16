import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ModelService } from "../src/model-service.ts";
import { emptyUsage } from "../src/model-bridge.ts";
import type { AssistantMessage, Context } from "@earendil-works/pi-ai";
const context: Context = {
  messages: [{ role: "user", content: "hello", timestamp: 0 }],
};
const response = (errorMessage?: string): AssistantMessage => ({
  role: "assistant",
  content: [],
  api: "openai-responses",
  provider: "openai",
  model: "fixture",
  timestamp: 0,
  stopReason: errorMessage ? "error" : "stop",
  errorMessage,
  usage: { ...emptyUsage(), input: 12, output: 3, totalTokens: 15 },
});
test("native model attempts reserve global budgets, count retries and settle only successful usage", async () => {
  const dir = await mkdtemp(join(tmpdir(), "gg-model-service-"));
  let attempts = 0;
  const service = new ModelService(
    dir,
    { provider: "openai", model: "fixture", maxOutputTokens: 100 },
    async (_m, _c, options) => {
      assert.equal(options?.maxRetries, 0);
      assert.equal(options?.maxTokens, 100);
      return ++attempts === 1 ? response("fetch failed") : response();
    },
  );
  let calls = 0;
  await service.complete(context, new AbortController().signal, () => calls++);
  assert.equal(attempts, 2);
  assert.equal(calls, 2);
  const status = await service.usage.status();
  assert.equal(status.windows[0].usedRequests, 2);
  assert.ok(status.windows[0].usedTokens > 115);
  const next = new ModelService(
    dir,
    { provider: "openai", model: "fixture", limits: { hourRequests: 2 } },
    async () => {
      throw Error("must not call provider");
    },
  );
  await assert.rejects(
    next.complete(context, new AbortController().signal, () => {}),
    /budget reached/,
  );
});
test("authentication failures pause across instances; oversized contexts never invoke providers", async () => {
  const dir = await mkdtemp(join(tmpdir(), "gg-model-pause-"));
  let calls = 0;
  const service = new ModelService(
    dir,
    { provider: "openai", model: "fixture" },
    async () => {
      calls++;
      return response("unauthorized");
    },
  );
  await assert.rejects(
    service.complete(context, new AbortController().signal, () => {}),
    /authentication/,
  );
  await assert.rejects(
    service.complete(context, new AbortController().signal, () => {}),
    /authentication/,
  );
  assert.equal(calls, 1);
  const small = new ModelService(
    dir,
    { provider: "openai", model: "fixture", maxInputTokens: 1 },
    async () => {
      throw Error("must not call");
    },
  );
  await assert.rejects(
    small.complete(context, new AbortController().signal, () => {}),
    /input exceeds/,
  );
});
