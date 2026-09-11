import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Vault } from "../src/vault.ts";
import sharp from "sharp";
import { createHash } from "node:crypto";

test("capture saves a readable record and source, and repeat capture returns its location", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-vault-test-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const draft = {
    kind: "idea" as const,
    title: "Triage customer messages",
    subvault: "Ideas",
    sourceUrl: "https://example.org/mail",
    summary:
      "Label incoming customer email by urgency. Draft replies for human review.",
    tags: ["support", "email"],
    sourceText:
      "Full source: classify messages, then review proposed responses.",
    publishedAt: "2026-05-27",
  };
  const first = await vault.save(draft);
  assert.equal(first.status, "saved");
  assert.match(
    await readFile(join(first.path!, "record.md"), "utf8"),
    /human review/,
  );
  assert.match(
    await readFile(join(first.path!, "source.md"), "utf8"),
    /Full source/,
  );
  const second = await vault.save(draft);
  assert.equal(second.status, "existing");
  assert.equal(first.path, second.path);
  assert.equal(
    (await readdir(join(root, "subvaults", "Ideas", "items"))).length,
    1,
  );
  assert.equal((await vault.search("customer email"))[0].title, draft.title);
});

test("destination traversal, missing subvaults and symlinked item directories cannot write outside the vault", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-path-test-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const draft = {
    kind: "idea" as const,
    title: "x",
    summary: "x",
    tags: [],
    sourceUrl: "https://example.org/x",
    sourceText: "x",
    subvault: "../outside",
  };
  await assert.rejects(vault.save(draft), /existing allowed/);
  await assert.rejects(
    vault.save({ ...draft, subvault: "New" }),
    /existing allowed/,
  );
  const outside = await mkdtemp(join(tmpdir(), "gg-outside-test-"));
  const id = "11111111-1111-4111-8111-111111111111";
  await symlink(outside, join(root, "subvaults", "Ideas", "items", id));
  assert.deepEqual(await vault.items(), []);
  assert.deepEqual(await readdir(outside), []);
});

test("ambiguous captures enter a durable queue; inaccessible reasons cannot enter it", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-queue-test-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const queued = await vault.queue(
    "https://example.org/art",
    "ambiguous-image",
    ["https://example.org/one.png", "https://example.org/two.png"],
  );
  assert.equal(queued.status, "queued");
  assert.equal((await readdir(join(root, "queue"))).length, 1);
  await assert.rejects(
    vault.queue("https://example.org/blocked", "login-required", []),
    /Invalid queue/,
  );
  assert.equal((await vault.items()).length, 0);
});

test("a conservative same-image upgrade preserves the earlier original; a different image cannot replace it", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-upgrade-test-"));
  const vault = await Vault.create(root, ["Paintings"]);
  async function image(width: number, color: string) {
    const bytes = await sharp({
      create: { width, height: width, channels: 3, background: color },
    })
      .png()
      .toBuffer();
    const visual = await sharp(bytes).resize(32, 32).raw().toBuffer();
    return {
      bytes: bytes.toString("base64"),
      width,
      height: width,
      visualHash: createHash("sha256").update(visual).digest("hex"),
    };
  }
  const original = await image(100, "#123456"),
    better = await image(200, "#123456"),
    different = await image(300, "#654321");
  const draft = {
    kind: "art" as const,
    title: "Square",
    summary: "A square fixture",
    tags: [],
    sourceUrl: "https://example.org/square",
    selectedImage: "https://example.org/square.png",
    subvault: "Paintings",
    assets: [original],
  };
  const first = await vault.save(draft);
  const second = await vault.save({ ...draft, assets: [better] });
  assert.equal(second.status, "upgraded");
  assert.equal(second.path, first.path);
  const [{ item }] = await vault.items();
  assert.equal(item.assets.length, 2);
  assert.match(item.primary!, /\.png$/);
  assert.equal((await readdir(join(first.path!, "files"))).length, 2);
  const rejected = await vault.save({ ...draft, assets: [different] });
  assert.equal(rejected.status, "existing");
  assert.equal((await vault.items())[0].item.primary, item.primary);
});

test("a competing writer is rejected without changing existing records", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-lock-test-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const { writeFile, unlink } = await import("node:fs/promises");
  await writeFile(join(root, ".write-lock"), "fixture owner");
  await assert.rejects(
    vault.save({
      kind: "idea",
      title: "x",
      summary: "x",
      sourceText: "x",
      tags: [],
      sourceUrl: "https://example.org/x",
      subvault: "Ideas",
    }),
    /writer is busy/,
  );
  assert.equal((await vault.items()).length, 0);
  await unlink(join(root, ".write-lock"));
});

test("a malformed record cannot hide valid searchable items", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-malformed-test-"));
  const vault = await Vault.create(root, ["Ideas"]);
  await vault.save({
    kind: "idea",
    title: "Customer email",
    summary: "Triage support mail",
    sourceText: "Actual source",
    tags: ["support"],
    sourceUrl: "https://example.org/valid",
    subvault: "Ideas",
  });
  const { mkdir, writeFile } = await import("node:fs/promises");
  const id = "11111111-1111-4111-8111-111111111111";
  const path = join(root, "subvaults", "Ideas", "items", id);
  await mkdir(path);
  await writeFile(
    join(path, "record.md"),
    `---\n${JSON.stringify({ id, title: "Broken" })}\n---\n`,
  );
  assert.equal((await vault.search("customer"))[0].title, "Customer email");
  assert.ok(vault.problems.has(path));
});

test("source reads can inspect a later match without returning an unbounded document", async () => {
  const vault = await Vault.create(
    await mkdtemp(join(tmpdir(), "gg-window-test-")),
    ["Ideas"],
  );
  await vault.save({
    kind: "idea",
    title: "Long source",
    summary: "A short summary",
    sourceText:
      "x".repeat(70000) +
      " Customer support requires human approval before sending.",
    tags: [],
    sourceUrl: "https://example.org/long",
    subvault: "Ideas",
  });
  const [hit] = await vault.search("customer");
  const first = await vault.read(hit.id);
  assert.equal(first.sourceText.length, 60000);
  assert.equal(first.nextOffset, 60000);
  const tail = await vault.read(hit.id, first.nextOffset!);
  assert.match(tail.sourceText, /human approval/);
  assert.equal(tail.nextOffset, null);
});

test("a copied vault retains working Obsidian links and searches user-edited YAML records", async () => {
  const { cp, writeFile, stat } = await import("node:fs/promises");
  const { relative, dirname, resolve } = await import("node:path");
  const root = await mkdtemp(join(tmpdir(), "gg-portable-test-"));
  const vault = await Vault.create(join(root, "original"), [
    "Ideas and advice",
  ]);
  const saved = await vault.save({
    kind: "idea",
    title: "Customer inbox",
    subvault: "Ideas and advice",
    sourceUrl: "https://example.org/guide",
    summary: "Classify customer messages.\n\n## Steps\n\n1. Review drafts.",
    sourceText: "Complete original instructions.",
    tags: ["email management"],
  });
  const record = join(saved.path!, "record.md");
  await writeFile(
    record,
    (await readFile(record, "utf8"))
      .replace(
        "Classify customer messages.",
        "My phrase: organize customer correspondence.",
      )
      .replace("---\n\n#", "rating: 5\n---\n\n#") +
      "\n## Personal notes\n\nUseful for my team.\n",
  );
  const relocated = join(root, "copied");
  await cp(vault.root, relocated, { recursive: true });
  const copy = await Vault.open(relocated);
  assert.equal(
    (await copy.search("correspondence"))[0].title,
    "Customer inbox",
  );
  const copiedRecord = join(relocated, relative(vault.root, record));
  for (const file of [join(relocated, "GG Index.md"), copiedRecord]) {
    const markdown = await readFile(file, "utf8");
    assert.ok(!markdown.includes(vault.root));
    for (const match of markdown.matchAll(/\]\(([^)]+)\)/g)) {
      if (/^https?:/.test(match[1])) continue;
      assert.ok(
        (await stat(resolve(dirname(file), decodeURI(match[1])))).isFile(),
      );
    }
  }
  assert.match(await readFile(copiedRecord, "utf8"), /rating: 5/);
  assert.match(await readFile(copiedRecord, "utf8"), /Useful for my team/);
});
