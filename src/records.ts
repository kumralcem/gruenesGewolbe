import { parseDocument, stringify } from "yaml";
import type { Item, StoredAsset } from "./types.ts";

const mediaStart = "<!-- gg:media -->",
  mediaEnd = "<!-- /gg:media -->";
const escapeLabel = (s: string) => s.replace(/[\[\]\\\n\r]/g, " ");
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
  const links = item.assets
    .map(
      (a) =>
        `- [${escapeLabel(a.file.split("/").at(-1)!)}](${encodeURI(a.file)})`,
    )
    .join("\n");
  return `${mediaStart}\n${item.primary ? `![${escapeLabel(item.title)}](${encodeURI(item.primary)})\n\n` : ""}${links ? `Preserved files:\n\n${links}\n\n` : ""}${item.kind === "idea" ? "[Preserved source](source.md)\n" : ""}${mediaEnd}`;
}
export function renderRecord(item: Item) {
  return `---\n${stringify(properties(item))}---\n\n# ${escapeLabel(item.title)}\n\n${media(item)}\n\n## Summary\n\n${item.summary}\n\n## Source\n\n[Original page](${encodeURI(item.sourceUrl)})\n`;
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
