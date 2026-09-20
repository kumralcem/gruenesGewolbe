import { test } from "node:test";
import assert from "node:assert/strict";
import { normalizeLocalAttribution } from "../src/attribution.ts";
import type { Draft } from "../src/types.ts";
test("filename metadata cannot masquerade as sourced facts and citations require fetched evidence", () => {
  const d: Draft = {
    kind: "art",
    title: "Studio",
    creator: "Someone",
    year: "1930",
    summary: "A painting",
    tags: [],
    subvault: "Art",
    sourceUrl: "gg-local:sha256:" + "a".repeat(64),
  };
  normalizeLocalAttribution(d);
  assert.equal(d.year, undefined);
  assert.equal(d.creator, undefined);
  assert.equal(d.attribution?.year?.status, "filename");
  const source = "https://museum.example/work";
  d.attribution = {
    year: {
      value: "1918",
      status: "source-supported",
      sourceUrl: source,
      quote: "Created 1918",
    },
  };
  assert.throws(() => normalizeLocalAttribution(d), /fetched/);
  normalizeLocalAttribution(d, new Map([[source, "<p>Created 1918</p>"]]));
  assert.equal(d.year, "1918");
});
