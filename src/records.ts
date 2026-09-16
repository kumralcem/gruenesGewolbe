import { parseDocument, stringify } from "yaml";
import type { Item, StoredAsset } from "./types.ts";

const mediaStart = "<!-- gg:media -->",
  mediaEnd = "<!-- /gg:media -->";
export const markdownLabel = (s: string) => s.replace(/[\[\]\\\n\r]/g, " ");
export const tagName = (s: string) =>
  s
    .trim()
    .replace(/^#+/, "")
    .replace(/[^\p{L}\p{N}_/-]+/gu, "-")
    .replace(/^-|-$/g, "")
    .replace(/\/{2,}/g, "/");
function split(source: string) {
  const match = source.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) throw Error("Missing YAML properties");
  const doc = parseDocument(match[1], { uniqueKeys: true });
  if (doc.errors.length) throw Error("Invalid YAML properties");
  const props = doc.toJS({ maxAliasCount: 20 });
  if (!props || typeof props !== "object" || Array.isArray(props))
    throw Error("Properties must be a mapping");
  return { doc, props, body: source.slice(match[0].length) };
}
function properties(item: Item) {
  return Object.fromEntries(
    Object.entries({
      gg_format: 2,
      captureKey: item.captureKey,
      instructions: item.instructions,
      missingMedia: item.missingMedia,
      id: item.id,
      kind: item.kind,
      title: item.title,
      aliases: [item.title],
      subvault: item.subvault,
      sourceUrl: item.sourceUrl,
      capturedAt: item.capturedAt,
      publishedAt: item.publishedAt || undefined,
      creator: item.creator || undefined,
      year: item.year || undefined,
      tags: [...new Set(item.tags.map(tagName).filter(Boolean))],
      selectedImage: item.selectedImage || undefined,
      primary: item.primary,
      files: item.assets.map((a) => a.file),
    }).filter(([, value]) => value !== undefined),
  );
}
function media(item: Item) {
  const embeds = (
    item.captureKey
      ? item.assets.map((a) => a.file)
      : item.primary
        ? [item.primary]
        : []
  )
    .map((file) => `![${markdownLabel(item.title)}](${encodeURI(file)})`)
    .join("\n\n");
  const omissions = item.missingMedia?.length
    ? `Capture notes:\n\n${item.missingMedia.map((m) => "- " + markdownLabel(m)).join("\n")}\n\n`
    : "";
  const links = item.assets
    .map(
      (a) =>
        `- [${markdownLabel(a.file.split("/").at(-1)!)}](${encodeURI(a.file)})`,
    )
    .join("\n");
  return `${mediaStart}\n${embeds ? embeds + "\n\n" : ""}${omissions}${links ? `Preserved files:\n\n${links}\n\n` : ""}${item.kind === "idea" || item.captureKey ? "[Preserved source](source.md)\n" : ""}${mediaEnd}`;
}
export function renderRecord(item: Item) {
  return `---\n${stringify(properties(item))}---\n\n# ${markdownLabel(item.title)}\n\n${media(item)}\n\n## Summary\n\n${item.summary}\n\n## Source\n\n[Original page](${encodeURI(item.sourceUrl)})\n`;
}
export function readRecord(source: string, assets?: StoredAsset[]): Item {
  const { props, body } = split(source);
  const summary =
    body
      .match(/(?:^|\n)## Summary\s*\n([\s\S]*?)(?=\n## Source\s*\n|$)/)?.[1]
      .trim() ?? props.summary;
  const item = { ...props, summary, assets: assets ?? props.assets ?? [] };
  if (
    !["art", "idea"].includes(item.kind) ||
    typeof item.id !== "string" ||
    !/^[-a-f0-9]{36}$/.test(item.id) ||
    typeof item.title !== "string" ||
    item.title.length > 300 ||
    typeof item.summary !== "string" ||
    item.summary.length > 20000 ||
    typeof item.sourceUrl !== "string" ||
    typeof item.capturedAt !== "string" ||
    !Array.isArray(item.tags) ||
    !item.tags.every((s: unknown) => typeof s === "string") ||
    !Array.isArray(item.assets) ||
    !item.assets.every(
      (a: any) =>
        a &&
        /^files\/[a-f0-9]{64}\.(jpg|png|webp)$/.test(a.file) &&
        /^[a-f0-9]{64}$/.test(a.hash) &&
        /^[a-f0-9]{64}$/.test(a.visualHash) &&
        Number.isSafeInteger(a.width) &&
        Number.isSafeInteger(a.height),
    )
  )
    throw Error("Malformed item record");
  return item as Item;
}
export function updateRecord(source: string, item: Item) {
  const { doc, body } = split(source);
  // Media upgrades only change managed file pointers. User properties, text and
  // notes remain authoritative; YAML is read even after Obsidian rewrites it.
  for (const key of ["primary", "files"]) doc.set(key, properties(item)[key]);
  doc.delete("assets");
  doc.set("gg_format", 2);
  const start = body.indexOf(mediaStart),
    end = body.indexOf(mediaEnd, start);
  const updated =
    start >= 0 && end >= 0
      ? body.slice(0, start) + media(item) + body.slice(end + mediaEnd.length)
      : body + "\n\n" + media(item) + "\n";
  return `---\n${doc.toString()}---\n${updated}`;
}

/** Three-way refresh: only replace generated fields still equal to their baseline. */
export function refreshRecord(
  source: string,
  baseline: Item | undefined,
  next: Item,
  note: string,
) {
  const { doc, body } = split(source);
  const current = readRecord(source, next.assets);
  const conflicts: string[] = [];
  const generated = properties(next),
    previous = baseline ? properties(baseline) : {};
  for (const key of [
    "kind",
    "title",
    "aliases",
    "subvault",
    "sourceUrl",
    "publishedAt",
    "creator",
    "year",
    "tags",
    "selectedImage",
    "captureKey",
    "instructions",
    "missingMedia",
  ]) {
    if (
      JSON.stringify(split(source).props[key]) === JSON.stringify(previous[key])
    ) {
      if (generated[key] === undefined) doc.delete(key);
      else doc.set(key, generated[key]);
    } else if (
      JSON.stringify(split(source).props[key]) !==
      JSON.stringify(generated[key])
    )
      conflicts.push(key);
  }
  let updated = body;
  if (baseline && current.title === baseline.title) {
    const heading = body.match(/^\s*# ([^\n]*)/)?.[1];
    if (heading === markdownLabel(baseline.title))
      updated = updated.replace(
        /^\s*# [^\n]*/,
        () => `\n# ${markdownLabel(next.title)}`,
      );
    else if (heading !== markdownLabel(next.title)) conflicts.push("heading");
  }
  if (baseline && current.summary === baseline.summary) {
    updated = updated.replace(
      /(## Summary\s*\n)[\s\S]*?(?=\n## Source\s*\n|$)/,
      (_all, heading) => `${heading}\n${next.summary}\n`,
    );
  } else if (current.summary !== next.summary) conflicts.push("summary");
  doc.set("primary", next.primary ?? null);
  doc.set(
    "files",
    next.assets.map((a) => a.file),
  );
  doc.set("gg_format", 2);
  const start = updated.indexOf(mediaStart),
    end = updated.indexOf(mediaEnd, start);
  if (start >= 0 && end >= 0)
    updated =
      updated.slice(0, start) +
      media(next) +
      updated.slice(end + mediaEnd.length);
  const change = `\n\n### ${new Date().toISOString()}\n\n${note}${conflicts.length ? ` Preserved manual edits: ${conflicts.join(", ")}. Proposed generated values are retained in .gg-baseline.json.` : ""}\n`;
  return { text: `---\n${doc.toString()}---\n${updated}${change}`, conflicts };
}

export function editRecord(
  source: string,
  patch: {
    title?: string;
    summary?: string;
    tags?: string[];
    subvault?: string;
  },
) {
  const { doc, body } = split(source);
  for (const key of ["title", "tags", "subvault"] as const)
    if (patch[key] !== undefined) doc.set(key, patch[key]);
  let updated = body;
  if (patch.title !== undefined) {
    doc.set("aliases", [patch.title]);
    updated = updated.replace(
      /^\s*# [^\n]*/,
      () => `\n# ${markdownLabel(patch.title!)}`,
    );
  }
  if (patch.summary !== undefined)
    updated = updated.replace(
      /(## Summary\s*\n)[\s\S]*?(?=\n## Source\s*\n|$)/,
      (_all, heading) => `${heading}\n${patch.summary}\n`,
    );
  return `---\n${doc.toString()}---\n${updated}\n\n<!-- GG management edit ${new Date().toISOString()} -->\n`;
}
