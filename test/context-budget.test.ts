import { test } from "node:test";
import assert from "node:assert/strict";
import type { Context } from "@earendil-works/pi-ai";
import {
  boundedCandidates,
  deduplicateContextImages,
} from "../src/context-budget.ts";
import { estimateInput } from "../src/model-service.ts";

test("repeated previews do not consume the input bound, and distinct images and text survive", () => {
  const context: Context = {
    messages: [
      {
        role: "user",
        content: [
          { type: "text", text: "Preserve this evidence" },
          ...Array.from({ length: 9 }, () => ({
            type: "image" as const,
            mimeType: "image/jpeg",
            data: "same-preview",
          })),
          { type: "image", mimeType: "image/jpeg", data: "different-preview" },
        ],
        timestamp: 0,
      },
    ],
  };
  assert.ok(estimateInput(context) > 64000);
  const bounded = deduplicateContextImages(context);
  assert.ok(estimateInput(bounded) < 20000);
  assert.equal(
    JSON.stringify(bounded).includes("Preserve this evidence"),
    true,
  );
  assert.equal(JSON.stringify(context).match(/same-preview/g)?.length, 9);
  assert.equal(JSON.stringify(bounded).match(/same-preview/g)?.length, 1);
  assert.equal(JSON.stringify(bounded).includes("different-preview"), true);
});

test("inline image data and oversized candidate lists cannot swamp page observations", () => {
  const candidates = [
    { url: "data:image/jpeg;base64," + "a".repeat(100000) },
    ...Array.from({ length: 100 }, (_, i) => ({
      url: "https://example.com/" + i + "?q=" + "x".repeat(900),
    })),
  ];
  const bounded = boundedCandidates(candidates);
  assert.ok(JSON.stringify(bounded).length < 5100);
  assert.ok(bounded.length > 0);
  assert.ok(bounded.every((c) => c.url.startsWith("https://")));
});
