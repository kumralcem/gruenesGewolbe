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

test("reset retains history, limits persist, and collection grants are scoped, bounded and revocable", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-usage-controls-"));
  let now = 100000000;
  const g = new UsageGuard(
    root,
    { hourRequests: 1, weekRequests: 2 },
    () => now,
  );
  await g.reserve("openai", 100, { job: "first", input: "original" });
  await assert.rejects(g.reserve("openai", 100), /hour request limit/);
  await g.reset("hour");
  assert.equal((await g.status()).windows[0].usedRequests, 0);
  assert.equal((await g.status()).windows[1].usedRequests, 1);
  await g.reserve("openai", 100, { job: "second" });
  await g.reset("hour");
  await assert.rejects(g.reserve("openai", 100), /week request limit/);
  assert.equal((await g.history()).jobs.length, 2);
  const source = "gg-local:sha256:" + "a".repeat(64);
  const id = await g.grant(2, 1000, 60, [source]);
  await assert.rejects(
    g.reserve("openai", 100, { grant: id, input: "https://other" }),
    /does not include/,
  );
  const a = await g.reserve("openai", 400, {
    grant: id,
    input: source,
    job: "import",
  });
  await g.settle(a, 100);
  assert.equal((await g.status()).grants[id].tokens, 900);
  await g.reserve("openai", 100, { grant: id, input: source, job: "import" });
  await assert.rejects(
    g.reserve("openai", 100, { grant: id, input: source }),
    /exhausted/,
  );
  await g.revokeGrant(id);
  await assert.rejects(
    g.reserve("openai", 100, { grant: id, input: source }),
    /missing/,
  );
  await g.reset("all");
  await g.setLimits({ hourRequests: 12 });
  assert.equal(
    (await new UsageGuard(root, {}, () => now).status()).windows[0].requests,
    12,
  );
  await g.failure("openai", "authentication");
  await g.reset("all");
  await assert.rejects(g.reserve("openai", 100), /authentication/);
  await g.resume("openai");
  const exp = await g.grant(2, 1000, 1, [source]);
  now += 60001;
  await assert.rejects(
    g.reserve("openai", 100, { grant: exp, input: source }),
    /expired/,
  );
});
