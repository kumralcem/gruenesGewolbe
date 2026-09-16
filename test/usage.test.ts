import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { UsageGuard, UsagePaused } from "../src/usage.ts";
test("global budgets persist, include failures, warn and pause across controller instances", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-usage-"));
  let now = 100000000;
  const limits = {
    hourRequests: 5,
    weekRequests: 10,
    hourTokens: 10000,
    weekTokens: 20000,
  };
  const guard = new UsageGuard(root, limits, () => now);
  for (let i = 0; i < 4; i++) await guard.reserve("openai", 100);
  assert.equal((await guard.status()).warning, true);
  await new UsageGuard(root, limits, () => now).reserve("openai", 100);
  await assert.rejects(guard.reserve("openai", 100), UsagePaused);
  await guard.override(1, 100, 60);
  await guard.reserve("openai", 100);
  await assert.rejects(guard.reserve("openai", 100), UsagePaused);
  now += 3600001;
  await guard.reserve("openai", 100);
  await guard.failure("openai", "authentication");
  await assert.rejects(guard.reserve("openai", 100), /authentication/);
  await guard.resume("openai");
  await guard.reserve("openai", 100);
});
