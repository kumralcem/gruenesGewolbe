import { test } from "node:test";
import assert from "node:assert/strict";
import { formatResult } from "../src/cli-output.ts";
test("normal output preserves actionable information without protocol traces", () => {
  const result = {
    exitCode: 0,
    answer: "Moved Photography.",
    batchId: "batch",
    management: [
      { operationId: "op" },
      {
        requiresConfirmation: true,
        proposalId: "proposal",
        preview: { title: "Delete record" },
      },
    ],
    usageWarnings: ["Near limit"],
    events: [{ type: "model_config", baseUrl: "internal" }],
  };
  const output = formatResult(result);
  assert.match(output, /Moved Photography/);
  assert.match(output, /gg undo batch/);
  assert.match(output, /Delete record/);
  assert.match(output, /gg confirm proposal/);
  assert.match(output, /Near limit/);
  assert.doesNotMatch(output, /model_config|internal/);
  assert.deepEqual(JSON.parse(formatResult(result, true)), result);
});
test("partial saves and failures remain visible even when the model claims success", () => {
  const output = formatResult({
    exitCode: 1,
    answer: "Done",
    outcomes: [{ status: "saved", path: "/record" }],
    outcome: { status: "partial", reason: "deadline exceeded" },
    stderr: "worker failed",
    vaultProblems: [{ path: "bad", reason: "invalid record" }],
    events: [{ type: "tool_error" }],
  });
  for (const detail of [
    "/record",
    "partial",
    "deadline exceeded",
    "worker failed",
    "invalid record",
    "tool attempt",
  ])
    assert.ok(output.includes(detail));
});
