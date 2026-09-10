import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, join, basename } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import sharp from "sharp";
import { Vault } from "../src/vault.ts";
import { runJob } from "../src/runner.ts";
import { fixtures } from "../src/fixtures.ts";

const files = process.argv.slice(2);
if (!files.length)
  throw Error("Pass image paths as arguments; originals are read only");
const root = resolve(".runs", `images-${Date.now()}`);
await mkdir(root, { recursive: true });
const reports = [];
const digest = (bytes: Buffer) =>
  createHash("sha256").update(bytes).digest("hex");
for (const [i, file] of files.entries()) {
  const vault = await Vault.create(join(root, String(i)), [
    "Paintings",
    "Ideas",
  ]);
  const original = await readFile(file);
  const result = await runJob({
    vault,
    config: {
      provider: "openai",
      model: "fixture-model",
      api: "openai-completions",
      maxSeconds: 90,
    },
    intent: "art",
    input: "https://fixtures.example/art",
    ...(await fixtures(file)),
  });
  assert.equal(result.outcome?.status, "saved", JSON.stringify(result));
  const [{ item, path }] = await vault.items();
  const stored = await readFile(join(path, item.primary!));
  assert.equal(digest(stored), digest(original));
  assert.equal(digest(await readFile(file)), digest(original));
  const preview = await readFile(join(path, "preview.jpg"));
  const metadata = await sharp(preview).metadata();
  assert.ok(metadata.width! <= 768 && metadata.height! <= 768);
  const report = {
    file: basename(file),
    bytes: original.length,
    width: item.assets[0].width,
    height: item.assets[0].height,
    previewBytes: preview.length,
    previewWidth: metadata.width,
    previewHeight: metadata.height,
    originalUnchanged: true,
    result: result.outcome,
  };
  reports.push(report);
  console.log(JSON.stringify(report));
}
await writeFile(join(root, "results.json"), JSON.stringify(reports, null, 2));
console.log(`Report: ${join(root, "results.json")}`);
