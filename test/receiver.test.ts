import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { createReceiver } from "../src/receiver.ts";
const capture = {
  version: 1,
  intent: "idea",
  url: "https://example.org/private",
  title: "Article",
  capturedAt: "2026-09-11T10:00:00Z",
  text: "Instructions from the signed-in page",
};
test("receiver requires pairing, rejects web origins, and processes an idempotent browser capture", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-receiver-test-")),
    ["Ideas"],
  );
  let calls = 0;
  const receiver = await createReceiver({
    vault,
    port: 0,
    processCapture: async (c) => {
      calls++;
      return {
        outcome: await vault.save({
          kind: "idea",
          title: c.title,
          summary: "Follow the instructions.",
          sourceText: c.text,
          subvault: "Ideas",
          sourceUrl: c.url,
          tags: [],
        }),
      };
    },
  });
  try {
    assert.equal((await fetch(receiver.url + "/captures")).status, 401);
    const headers = {
      authorization: `Bearer ${receiver.token}`,
      "content-type": "application/json",
      "x-gg-capture-id": "11111111-1111-4111-8111-111111111111",
    };
    assert.equal(
      (
        await fetch(receiver.url + "/captures", {
          method: "POST",
          headers: { ...headers, origin: "https://evil.example" },
          body: JSON.stringify(capture),
        })
      ).status,
      403,
    );
    const first = await fetch(receiver.url + "/captures", {
      method: "POST",
      headers,
      body: JSON.stringify(capture),
    });
    assert.equal(first.status, 202);
    const { id } = await first.json();
    assert.equal(
      (
        await fetch(receiver.url + "/captures", {
          method: "POST",
          headers,
          body: JSON.stringify(capture),
        })
      ).status,
      202,
    );
    await receiver.idle();
    const record = await (
      await fetch(receiver.url + "/captures/" + id, { headers })
    ).json();
    assert.equal(record.status, "completed");
    assert.equal(record.result.outcome.status, "saved");
    assert.equal(calls, 1);
    assert.equal((await vault.items()).length, 1);
  } finally {
    await receiver.close();
  }
});

test("receiver keeps interrupted and pending inputs across restarts, retries once, and localizes malformed jobs", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-recovery-test-")),
    ["Ideas"],
  );
  let started!: () => void;
  const running = new Promise<void>((resolve) => {
    started = resolve;
  });
  let receiver = await createReceiver({
    vault,
    port: 0,
    processCapture: async (_, signal) => {
      started();
      await new Promise((_, reject) =>
        signal.addEventListener("abort", () => reject(Error("Interrupted")), {
          once: true,
        }),
      );
      return { outcome: { status: "skipped" } };
    },
  });
  const submit = async (id: string) => {
    const response = await fetch(receiver.url + "/captures", {
      method: "POST",
      headers: {
        authorization: `Bearer ${receiver.token}`,
        "content-type": "application/json",
        "x-gg-capture-id": id,
      },
      body: JSON.stringify(capture),
    });
    assert.equal(response.status, 202);
  };
  const first = "11111111-1111-4111-8111-111111111111",
    second = "22222222-2222-4222-8222-222222222222";
  try {
    await submit(first);
    await running;
    await submit(second);
  } finally {
    await receiver.close();
  }
  const { mkdir, writeFile, access } = await import("node:fs/promises");
  await mkdir(
    join(vault.root, ".gg-jobs", "33333333-3333-4333-8333-333333333333"),
  );
  await writeFile(
    join(
      vault.root,
      ".gg-jobs",
      "33333333-3333-4333-8333-333333333333",
      "job.json",
    ),
    "broken",
  );
  let calls = 0;
  receiver = await createReceiver({
    vault,
    port: 0,
    processCapture: async () => {
      calls++;
      return { outcome: { status: "skipped" } };
    },
  });
  try {
    await receiver.idle();
    assert.equal(calls, 1, "only the pending capture restarts automatically");
    assert.equal(receiver.problems.length, 1);
    const headers = { authorization: `Bearer ${receiver.token}` };
    const interrupted = await (
      await fetch(receiver.url + "/captures/" + first, { headers })
    ).json();
    assert.equal(interrupted.status, "interrupted");
    const retries = await Promise.all(
      [1, 2].map(() =>
        fetch(receiver.url + "/captures/" + first + "/retry", {
          method: "POST",
          headers,
        }),
      ),
    );
    assert.ok(retries.some((r) => r.status === 202));
    await receiver.idle();
    assert.equal(calls, 2, "concurrent retry cannot run the same input twice");
    await assert.rejects(
      access(join(vault.root, ".gg-jobs", first, "input.json")),
      { code: "ENOENT" },
    );
    await submit(first);
    await receiver.idle();
    assert.equal(
      calls,
      2,
      "completed capture ID still deduplicates after restart",
    );
    const changed = await fetch(receiver.url + "/captures", {
      method: "POST",
      headers: {
        ...headers,
        "content-type": "application/json",
        "x-gg-capture-id": first,
      },
      body: JSON.stringify({ ...capture, text: "Different" }),
    });
    assert.equal(changed.status, 400);
  } finally {
    await receiver.close();
  }
});
