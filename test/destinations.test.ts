import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Vault } from "../src/vault.ts";
test("an already-open vault discovers manually created nested destinations and saves searchable records there", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-destinations-"));
  const vault = await Vault.create(root, ["Photography"]);
  await mkdir(join(root, "subvaults", "Photography", "Historic (写真)"));
  await vault.items();
  assert.ok(vault.areas.includes("Photography/Historic (写真)"));
  const saved = await vault.capture(
    {
      kind: "idea",
      title: "Early photography",
      summary: "Historic (写真)al photographic methods",
      sourceText: "Original source",
      sourceUrl: "https://example.com/history",
      subvault: "Photography/Historic (写真)",
      tags: [],
    },
    "nested",
  );
  assert.equal(saved.status, "saved");
  assert.ok(saved.path?.includes("Photography/Historic (写真)/items/"));
  assert.equal((await vault.search("photographic"))[0].path, saved.path);
  const index = await readFile(join(root, "GG Index.md"), "utf8");
  assert.match(
    index,
    /subvaults\/Photography\/Historic%20%28%E5%86%99%E7%9C%9F%29\/items\//,
  );
  assert.ok(
    (await Vault.open(root)).areas.includes("Photography/Historic (写真)"),
  );
});

test("nested destination management renames descendants and undo restores empty folders", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-nested-manage-"));
  const vault = await Vault.create(root, ["Photography"]);
  const created = await vault.manage({
    action: "create-subvault",
    subvault: "Photography/Historic",
  });
  await vault.undo(created.operationId);
  await vault.items();
  assert.ok(!vault.areas.includes("Photography/Historic"));
  await vault.manage({
    action: "create-subvault",
    subvault: "Photography/Historic",
  });
  await mkdir(join(root, "subvaults", "Photography", "Empty"));
  await vault.capture(
    {
      kind: "idea",
      title: "Historic",
      summary: "old photos",
      sourceText: "original",
      sourceUrl: "https://example.com/old",
      tags: [],
      subvault: "Photography/Historic",
    },
    "r1",
  );
  const renamed = await vault.manage({
    action: "rename-subvault",
    from: "Photography",
    subvault: "Photos",
  });
  assert.equal((await vault.items())[0].item.subvault, "Photos/Historic");
  await vault.undo(renamed.operationId);
  assert.equal((await vault.items())[0].item.subvault, "Photography/Historic");
  assert.ok(vault.areas.includes("Photography/Empty"));
  assert.ok(!vault.areas.includes("Photos"));
});

test("discovery ignores private folders, record internals and symlinks; unsafe destinations cannot write outside", async () => {
  const { symlink, readdir } = await import("node:fs/promises");
  const root = await mkdtemp(join(tmpdir(), "gg-discovery-safe-")),
    outside = await mkdtemp(join(tmpdir(), "gg-outside-"));
  const vault = await Vault.create(root, ["Photography"]);
  await mkdir(join(root, "subvaults", "Photography", ".hidden"));
  await mkdir(
    join(root, "subvaults", "Photography", "items", "looks-like-a-category"),
  );
  await symlink(outside, join(root, "subvaults", "Photography", "Linked"));
  await mkdir(join(root, "subvaults", "Photography", "Époque ancienne"));
  const destinations = await vault.destinations();
  assert.ok(destinations.includes("Photography/Époque ancienne"));
  assert.ok(!destinations.some((a) => /hidden|looks-like|Linked/.test(a)));
  for (const subvault of [
    "Photography/../Escape",
    "Photography/Linked/Escape",
    "/Escape",
    "Photography/items/Bad",
  ])
    await assert.rejects(vault.manage({ action: "create-subvault", subvault }));
  assert.deepEqual(await readdir(outside), []);
  const created = await vault.manage({
    action: "create-subvault",
    subvault: "Photography/Later",
  });
  await mkdir(join(root, "subvaults", "Photography", "Later", "Human folder"));
  await assert.rejects(vault.undo(created.operationId), /later additions/);
});

test("parent renames cannot hide descendants by exceeding path limits", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-nested-limits-"));
  const vault = await Vault.create(root, ["A", "B"]);
  const deepest = "A/" + Array(15).fill("x").join("/");
  await mkdir(join(root, "subvaults", deepest), { recursive: true });
  await assert.rejects(
    vault.manage({ action: "rename-subvault", from: "A", subvault: "B/A" }),
    /path limits/,
  );
  assert.ok((await vault.destinations()).includes(deepest));
});
