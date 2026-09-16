import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  readFile,
  writeFile,
  readdir,
  symlink,
} from "node:fs/promises";
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
    title: "Customer inbox \\",
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
    "Customer inbox \\",
  );
  const copiedRecord = join(relocated, relative(vault.root, record));
  for (const file of [join(relocated, "GG Index.md"), copiedRecord]) {
    const markdown = await readFile(file, "utf8");
    assert.ok(!markdown.includes(vault.root));
    assert.ok(
      !markdown.includes("\\]("),
      "a backslash in the title must not escape the link delimiter",
    );
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

test("agent captures update in Inbox, preserve user edits, and can be undone", async () => {
  const { writeFile } = await import("node:fs/promises");
  const root = await mkdtemp(join(tmpdir(), "gg-refresh-"));
  const vault = await Vault.create(root, ["Ideas"]);
  const draft = {
    kind: "idea" as const,
    title: "A page",
    summary: "First summary",
    sourceText: "Original text",
    sourceUrl: "https://example.org/page",
    subvault: "Unknown",
    tags: [],
  };
  const first = await vault.capture(draft, "v1", "batch1");
  assert.match(first.path!, /Inbox/);
  const file = join(first.path!, "record.md");
  await writeFile(
    file,
    (await readFile(file, "utf8")).replace(
      "First summary",
      "My edited summary",
    ) + "\n## My notes\n\nKeep this.\n",
  );
  const second = await vault.capture(
    { ...draft, title: "New title", summary: "Second summary" },
    "v2",
    "batch2",
  );
  assert.equal(second.path, first.path);
  const record = await readFile(file, "utf8");
  assert.match(record, /My edited summary/);
  assert.match(record, /Keep this/);
  assert.match(record, /New title/);
  assert.deepEqual(second.conflicts, ["summary"]);
  assert.equal((await vault.items()).length, 1);
  await vault.undo(second.operationId);
  assert.match(await readFile(file, "utf8"), /title: A page/);
});
test("management previews deletion, rejects stale approvals, and reverses moves", async () => {
  const { writeFile } = await import("node:fs/promises");
  const root = await mkdtemp(join(tmpdir(), "gg-manage-"));
  const vault = await Vault.create(root, ["Ideas", "Art"]);
  const saved = await vault.capture(
    {
      kind: "idea",
      title: "Example",
      summary: "Hello",
      tags: [],
      sourceUrl: "https://example.org/a",
      sourceText: "Source",
      subvault: "Ideas",
    },
    "1",
  );
  const id = (await vault.items())[0].item.id;
  const moved = await vault.manage(
    { action: "move", id, subvault: "Art" },
    "batch",
  );
  assert.match((await vault.read(id)).path, /Art/);
  await vault.undo(moved.operationId);
  assert.equal((await vault.read(id)).path, saved.path);
  const preview = await vault.manage({ action: "delete", id });
  assert.equal(preview.requiresConfirmation, true);
  assert.equal((await vault.items()).length, 1);
  await writeFile(
    join(saved.path!, "record.md"),
    (await readFile(join(saved.path!, "record.md"), "utf8")) + "\nUser edit\n",
  );
  await assert.rejects(vault.confirm(preview.proposalId!), /changed/);
  const latest = await vault.manage({ action: "delete", id });
  const deleted = await vault.confirm(latest.proposalId!);
  assert.equal((await vault.items()).length, 0);
  await vault.undo(deleted.operationId);
  assert.equal((await vault.items()).length, 1);
});

test("separate vault instances preserve destination changes and manually edited headings", async () => {
  const root = await mkdtemp(join(tmpdir(), "gg-multi-instance-"));
  const first = await Vault.create(root, ["Ideas"]),
    second = await Vault.open(root);
  await first.manage({ action: "create-subvault", subvault: "Architecture" });
  await second.manage({ action: "create-subvault", subvault: "Design" });
  assert.deepEqual((await Vault.open(root)).areas, [
    "Ideas",
    "Architecture",
    "Design",
  ]);
  const draft = {
    kind: "idea" as const,
    title: "Generated",
    summary: "Useful",
    sourceText: "Original",
    subvault: "Architecture",
    sourceUrl: "https://example.com/new",
    tags: [],
  };
  const saved = await first.capture(draft, "first");
  const file = join(saved.path!, "record.md");
  await writeFile(
    file,
    (await readFile(file, "utf8")).replace("# Generated", "# My own heading"),
  );
  const updated = await second.capture(
    { ...draft, title: "New generated title" },
    "second",
  );
  assert.ok(updated.conflicts?.includes("heading"));
  assert.match(await readFile(file, "utf8"), /# My own heading/);
  assert.equal((await first.items()).length, 1);
});
