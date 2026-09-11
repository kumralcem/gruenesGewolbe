import {
  lstat,
  mkdir,
  readFile,
  readdir,
  rename,
  writeFile,
  realpath,
  unlink,
  rm,
} from "node:fs/promises";
import { join } from "node:path";
import { createHash, randomUUID } from "node:crypto";
import type { Asset, Draft, Hit, Item, Outcome, StoredAsset } from "./types.ts";
import { renderRecord, readRecord, updateRecord } from "./records.ts";

export const sha = (data: string | Buffer) =>
  createHash("sha256").update(data).digest("hex");
const marker = ".gg-vault.json";
const text = (s: unknown, max = 20000): s is string =>
  typeof s === "string" && s.length <= max;
const name = (s: string) => /^[a-zA-Z][a-zA-Z0-9 _-]{0,60}$/.test(s);
function validateUrl(value: string) {
  const u = new URL(value);
  if (!["http:", "https:"].includes(u.protocol) || u.username || u.password)
    throw Error("Invalid source URL");
}
async function plain(path: string, directory = false) {
  const s = await lstat(path);
  if (s.isSymbolicLink() || (directory ? !s.isDirectory() : !s.isFile()))
    throw Error("Unsafe filesystem entry");
}
function decodeAsset(a: Asset) {
  if (!text(a.bytes, 90_000_000) || !/^[A-Za-z0-9+/]*={0,2}$/.test(a.bytes))
    throw Error("Invalid image bytes");
  const bytes = Buffer.from(a.bytes, "base64");
  let ext = "";
  if (bytes.subarray(0, 3).equals(Buffer.from([255, 216, 255]))) ext = "jpg";
  if (
    bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
  )
    ext = "png";
  if (
    bytes.toString("ascii", 0, 4) === "RIFF" &&
    bytes.toString("ascii", 8, 12) === "WEBP"
  )
    ext = "webp";
  if (
    !ext ||
    bytes.length > 64_000_000 ||
    !Number.isSafeInteger(a.width) ||
    !Number.isSafeInteger(a.height) ||
    a.width < 1 ||
    a.height < 1 ||
    a.width * a.height > 100_000_000 ||
    !/^[a-f0-9]{64}$/.test(a.visualHash)
  )
    throw Error("Unsupported or unbounded image");
  return { bytes, ext, hash: sha(bytes) };
}
export class Vault {
  readonly problems = new Map<string, string>();
  private pending: Promise<unknown> = Promise.resolve();
  private constructor(
    readonly root: string,
    readonly areas: string[],
  ) {}
  static async create(root: string, areas: string[]) {
    if (
      !areas.length ||
      !areas.every(name) ||
      new Set(areas).size !== areas.length
    )
      throw Error("Invalid subvault names");
    await mkdir(root, { recursive: true });
    await plain(root, true);
    if ((await readdir(root)).length)
      throw Error("Vault init requires an empty directory");
    const canonical = await realpath(root);
    await writeFile(
      join(canonical, marker),
      JSON.stringify({ format: 2, areas }),
      { flag: "wx", mode: 0o600 },
    );
    await mkdir(join(canonical, "subvaults"));
    for (const area of areas)
      await mkdir(join(canonical, "subvaults", area, "items"), {
        recursive: true,
      });
    await mkdir(join(canonical, "queue"));
    await mkdir(join(canonical, ".staging"));
    const vault = new Vault(canonical, areas);
    await vault.index();
    return vault;
  }
  static async open(root: string) {
    await plain(root, true);
    const current = await lstat(join(root, marker)).then(
      () => marker,
      (e: NodeJS.ErrnoException) => {
        if (e.code === "ENOENT") return ".gg-prototype.json";
        throw e;
      },
    );
    await plain(join(root, current));
    const data = JSON.parse(await readFile(join(root, current), "utf8"));
    if (
      (data.format !== 2 && data.prototype !== true) ||
      !Array.isArray(data.areas) ||
      !data.areas.every((a: unknown) => typeof a === "string" && name(a))
    )
      throw Error("Not a GG vault");
    return new Vault(await realpath(root), data.areas);
  }
  private async areaPath(area: string) {
    if (!this.areas.includes(area))
      throw Error("Destination is not an existing allowed subvault");
    for (const p of [
      "subvaults",
      `subvaults/${area}`,
      `subvaults/${area}/items`,
    ])
      await plain(join(this.root, p), true);
    return join(this.root, "subvaults", area, "items");
  }
  async items(): Promise<{ item: Item; path: string }[]> {
    const results: { item: Item; path: string }[] = [];
    for (const area of this.areas) {
      const folder = await this.areaPath(area);
      for (const dir of await readdir(folder)) {
        if (!/^[a-f0-9-]{36}$/.test(dir)) continue;
        const path = join(folder, dir);
        try {
          await plain(path, true);
          await plain(join(path, "record.md"));
          let assets: StoredAsset[] | undefined;
          try {
            await plain(join(path, ".gg-assets.json"));
            assets = JSON.parse(
              await readFile(join(path, ".gg-assets.json"), "utf8"),
            );
          } catch (e) {
            if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
          }
          const item = readRecord(
            await readFile(join(path, "record.md"), "utf8"),
            assets,
          );
          if (item.id === dir) results.push({ item, path });
          this.problems.delete(path);
        } catch (error) {
          this.problems.set(
            path,
            error instanceof Error ? error.message : String(error),
          );
        }
      }
    }
    return results;
  }
  async read(id: string, offset = 0) {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset > 400000)
      throw Error("Invalid source offset");
    const found = (await this.items()).find((v) => v.item.id === id);
    if (!found) throw Error("Unknown item ID");
    let sourceText = "",
      totalCharacters = 0;
    if (found.item.kind === "idea") {
      const full = await this.source(found.path);
      totalCharacters = full.length;
      sourceText = full.slice(offset, offset + 60000);
    }
    return {
      ...found,
      sourceText,
      offset,
      totalCharacters,
      nextOffset:
        offset + sourceText.length < totalCharacters
          ? offset + sourceText.length
          : null,
    };
  }
  async search(query: string): Promise<Hit[]> {
    if (!text(query, 1000)) throw Error("Invalid query");
    const terms = query.toLocaleLowerCase().match(/[\p{L}\p{N}]{2,}/gu) ?? [];
    const hits: Hit[] = [];
    for (const { item, path } of await this.items()) {
      const head =
        `${item.title} ${item.tags.join(" ")} ${item.creator ?? ""}`.toLocaleLowerCase();
      let content = item.summary;
      if (item.kind === "idea") {
        try {
          content += "\n" + (await this.source(path));
        } catch {
          /* Search the valid record. */
        }
      }
      const lower = content.toLocaleLowerCase();
      const score = terms.reduce(
        (n, t) => n + (head.includes(t) ? 4 : 0) + (lower.includes(t) ? 1 : 0),
        0,
      );
      if (score) {
        const at = Math.max(
          0,
          lower.indexOf(terms.find((t) => lower.includes(t)) ?? "") - 80,
        );
        hits.push({
          id: item.id,
          title: item.title,
          path,
          snippet: content.slice(at, at + 650),
          score,
          publishedAt: item.publishedAt,
          capturedAt: item.capturedAt,
        });
      }
    }
    return hits.sort((a, b) => b.score - a.score).slice(0, 8);
  }
  save(draft: Draft): Promise<Outcome> {
    const next = this.pending.then(() =>
      this.withWriteLock(async () => {
        const result = await this.saveOne(draft);
        await this.refreshIndex().catch((error) => {
          this.problems.set(
            join(this.root, "GG Index.md"),
            `Item saved; index refresh failed: ${String(error)}`,
          );
        });
        return result;
      }),
    );
    this.pending = next.catch(() => {});
    return next;
  }
  private async withWriteLock<T>(write: () => Promise<T>): Promise<T> {
    const path = join(this.root, ".write-lock");
    await writeFile(
      path,
      JSON.stringify({ pid: process.pid, createdAt: new Date().toISOString() }),
      { flag: "wx", mode: 0o600 },
    ).catch((e: NodeJS.ErrnoException) => {
      if (e.code === "EEXIST")
        throw Error(
          "Vault writer is busy, or an interrupted writer left .write-lock; retry after checking it",
        );
      throw e;
    });
    try {
      return await write();
    } finally {
      await unlink(path);
    }
  }
  private async saveOne(d: Draft): Promise<Outcome> {
    if (
      !["idea", "art"].includes(d.kind) ||
      !text(d.title, 300) ||
      !d.title.trim() ||
      !text(d.summary) ||
      !Array.isArray(d.tags) ||
      d.tags.length > 30 ||
      !d.tags.every((t) => text(t, 80)) ||
      !text(d.sourceUrl, 4000) ||
      !text(d.sourceText ?? "", 400000)
    )
      throw Error("Invalid capture record");
    validateUrl(d.sourceUrl);
    if (d.publishedAt && !/^\d{4}-\d{2}-\d{2}$/.test(d.publishedAt))
      throw Error("Invalid publication date");
    if (d.kind === "idea" && (!d.sourceText?.trim() || !d.summary.trim()))
      throw Error("Ideas need source text and a summary");
    const parent = await this.areaPath(d.subvault);
    const assets = d.assets ?? [];
    if (!Array.isArray(assets) || (d.kind === "idea" && assets.length))
      throw Error("Ideas cannot include image assets");
    if (
      (d.creator !== undefined && !text(d.creator, 300)) ||
      (d.year !== undefined && !text(d.year, 80))
    )
      throw Error("Invalid attribution");
    if (
      assets.length > 2 ||
      (d.kind === "art" && (!assets.length || !d.selectedImage))
    )
      throw Error(
        "Visual capture requires a selected image and at most two copies",
      );
    if (d.selectedImage) validateUrl(d.selectedImage);
    const decoded = assets.map(decodeAsset);
    if (assets.length === 2 && assets[0].visualHash !== assets[1].visualHash)
      throw Error(
        "Cannot confirm that the second copy represents the same selected image; submit the original alone",
      );
    for (const a of assets)
      if (a.preview) {
        const preview = Buffer.from(a.preview, "base64");
        if (
          !text(a.preview, 2_000_000) ||
          !preview.subarray(0, 3).equals(Buffer.from([255, 216, 255]))
        )
          throw Error("Invalid JPEG preview");
      }
    const stored: StoredAsset[] = decoded.map((a, i) => ({
      file: `files/${a.hash}.${a.ext}`,
      hash: a.hash,
      width: assets[i].width,
      height: assets[i].height,
      visualHash: assets[i].visualHash,
    }));
    const candidates = await this.items();
    const existing = candidates.find(
      ({ item }) =>
        item.kind === d.kind &&
        (d.kind === "idea"
          ? item.sourceUrl === d.sourceUrl
          : item.selectedImage === d.selectedImage ||
            item.assets.some((a) => decoded.some((b) => b.hash === a.hash))),
    );
    let best = stored[0];
    for (const a of stored.slice(1))
      if (
        a.visualHash === best.visualHash &&
        a.width >= best.width &&
        a.height >= best.height &&
        a.width * a.height > best.width * best.height
      )
        best = a;
    if (existing) {
      const old = existing.item.assets.find(
        (a) => a.file === existing.item.primary,
      );
      if (
        !old ||
        !best ||
        old.visualHash !== best.visualHash ||
        best.width < old.width ||
        best.height < old.height ||
        best.width * best.height <= old.width * old.height
      )
        return { status: "existing", path: existing.path };
      await plain(join(existing.path, "files"), true);
      for (let i = 0; i < stored.length; i++)
        if (!existing.item.assets.some((a) => a.hash === stored[i].hash)) {
          await writeFile(
            join(existing.path, stored[i].file),
            decoded[i].bytes,
            { flag: "wx" },
          );
          existing.item.assets.push(stored[i]);
        }
      existing.item.primary = best.file;
      const temp = join(existing.path, `.record-${randomUUID()}`);
      const oldRecord = await readFile(
        join(existing.path, "record.md"),
        "utf8",
      );
      const assetTemp = join(existing.path, `.assets-${randomUUID()}`);
      await writeFile(assetTemp, JSON.stringify(existing.item.assets, null, 2));
      await rename(assetTemp, join(existing.path, ".gg-assets.json"));
      await writeFile(temp, updateRecord(oldRecord, existing.item), {
        flag: "wx",
      });
      await rename(temp, join(existing.path, "record.md"));
      return { status: "upgraded", path: existing.path };
    }
    const id = randomUUID();
    await plain(join(this.root, ".staging"), true);
    const stage = join(this.root, ".staging", id);
    await mkdir(stage);
    try {
      const item: Item = {
        id,
        kind: d.kind,
        title: d.title,
        summary: d.summary,
        tags: d.tags,
        sourceUrl: d.sourceUrl,
        subvault: d.subvault,
        capturedAt: new Date().toISOString(),
        publishedAt: d.publishedAt,
        creator: d.creator,
        year: d.year,
        selectedImage: d.selectedImage,
        assets: stored,
        primary: best?.file,
      };
      if (stored.length) {
        await mkdir(join(stage, "files"));
        for (let i = 0; i < stored.length; i++)
          await writeFile(join(stage, stored[i].file), decoded[i].bytes);
        if (assets[0].preview)
          await writeFile(
            join(stage, "preview.jpg"),
            Buffer.from(assets[0].preview, "base64"),
          );
      }
      if (d.kind === "idea")
        await writeFile(join(stage, "source.md"), d.sourceText!);
      await writeFile(
        join(stage, ".gg-assets.json"),
        JSON.stringify(item.assets, null, 2),
      );
      await writeFile(join(stage, "record.md"), renderRecord(item));
      const path = join(parent, id);
      await rename(stage, path);
      return { status: "saved", path };
    } finally {
      await rm(stage, { recursive: true, force: true });
    }
  }
  private async source(path: string) {
    let file = join(path, "source.md");
    try {
      await plain(file);
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
      file = join(path, "source.txt");
      await plain(file);
    }
    return readFile(file, "utf8");
  }
  async index() {
    return this.withWriteLock(() => this.refreshIndex());
  }
  private async refreshIndex() {
    const target = join(this.root, "GG Index.md");
    const entries = await this.items();
    const lines = [
      "<!-- generated by GG: index -->",
      "# GG Archive",
      "",
      "Open this folder as an Obsidian vault. Each item has a readable record and local files.",
      "",
    ];
    for (const area of this.areas) {
      lines.push(`## ${area}`, "");
      for (const { item } of entries.filter((e) => e.item.subvault === area)) {
        const label = item.title.replace(/[\[\]\r\n]/g, " ");
        lines.push(
          `- [${label}](subvaults/${encodeURIComponent(area)}/items/${item.id}/record.md)`,
        );
      }
      lines.push("");
    }
    try {
      await plain(target);
      if (
        !(await readFile(target, "utf8")).startsWith(
          "<!-- generated by GG: index -->",
        )
      ) {
        this.problems.set(
          target,
          "Index was replaced with a user note; not overwritten",
        );
        return target;
      }
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
    }
    const temp = join(this.root, `.index-${randomUUID()}`);
    await writeFile(temp, lines.join("\n"));
    await rename(temp, target);
    return target;
  }
  async queue(sourceUrl: string, reason: string, candidates: string[]) {
    return this.withWriteLock(() =>
      this.queueOne(sourceUrl, reason, candidates),
    );
  }
  private async queueOne(
    sourceUrl: string,
    reason: string,
    candidates: string[],
  ) {
    validateUrl(sourceUrl);
    if (
      !["ambiguous-image", "no-subvault"].includes(reason) ||
      !Array.isArray(candidates) ||
      candidates.length > 20 ||
      !candidates.every((v) => text(v, 4000))
    )
      throw Error("Invalid queue request");
    await plain(join(this.root, "queue"), true);
    const path = join(this.root, "queue", `${sha(sourceUrl + reason)}.json`);
    await writeFile(
      path,
      JSON.stringify(
        { sourceUrl, reason, candidates, createdAt: new Date().toISOString() },
        null,
        2,
      ),
      { flag: "wx" },
    ).catch((e: NodeJS.ErrnoException) => {
      if (e.code !== "EEXIST") throw e;
    });
    return { status: "queued", path, reason } satisfies Outcome;
  }
}
