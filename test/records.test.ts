import { test } from "node:test";
import assert from "node:assert/strict";
import { parse } from "yaml";
import { renderRecord, readRecord, updateRecord } from "../src/records.ts";
import type { Item } from "../src/types.ts";
const item: Item = {
  id: "11111111-1111-4111-8111-111111111111",
  kind: "art",
  title: "Flowers: a study",
  summary: "Oil painting of flowers.",
  subvault: "Paintings",
  sourceUrl: "https://example.org/art",
  capturedAt: "2026-09-11T08:00:00Z",
  tags: ["Oil Painting", "19th century"],
  assets: [],
  primary: "files/abc.jpg",
};
test("records use Obsidian properties and portable local media links", () => {
  const text = renderRecord(item);
  const properties = parse(text.split("---")[1]);
  assert.deepEqual(properties.aliases, ["Flowers: a study"]);
  assert.deepEqual(properties.tags, ["Oil-Painting", "19th-century"]);
  assert.equal(properties.title, item.title);
  assert.equal(properties.assets, undefined);
  assert.equal(properties.summary, undefined);
  assert.match(text, /!\[.*\]\(files\/abc.jpg\)/);
  assert.match(text, /## Summary\n\nOil painting/);
  assert.equal(readRecord(text).title, item.title);
});
test("upgrading metadata preserves Obsidian properties, notes and edited summary", () => {
  const text =
    renderRecord(item)
      .replace("Oil painting of flowers.", "My edited description.")
      .replace("---\n\n#", "rating: 5\n---\n\n#") +
    "\n## My notes\n\nKeep this note.\n";
  const updated = updateRecord(text, { ...item, primary: "files/better.jpg" });
  assert.match(updated, /rating: 5/);
  assert.match(updated, /My edited description/);
  assert.match(updated, /Keep this note/);
  assert.match(updated, /files\/better.jpg/);
});

test("instruction headings remain part of the searchable summary", () => {
  const summary =
    "Classify email.\n\n## Steps\n\n1. Export messages.\n2. Review drafts.";
  assert.equal(
    readRecord(renderRecord({ ...item, kind: "idea", summary })).summary,
    summary,
  );
});
