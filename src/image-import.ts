import { createHash } from "node:crypto";
import { constants } from "node:fs";
import { lstat, readdir, open } from "node:fs/promises";
import { basename, extname, join, resolve } from "node:path";
import {
  validateBrowserCapture,
  type BrowserCapture,
} from "./browser-capture.ts";

const extensions = new Set([".jpg", ".jpeg", ".png", ".webp"]);
export async function* imageFiles(paths: string[]): AsyncGenerator<string> {
  const visited = new Set<string>();
  async function* walk(
    path: string,
    explicit: boolean,
  ): AsyncGenerator<string> {
    path = resolve(path);
    if (visited.has(path)) return;
    visited.add(path);
    const stat = await lstat(path);
    if (stat.isSymbolicLink()) {
      if (explicit) throw Error(`Symbolic links are not imported: ${path}`);
      return;
    }
    if (stat.isDirectory()) {
      for (const name of (await readdir(path)).sort()) {
        if (!name.startsWith(".")) yield* walk(join(path, name), false);
      }
    } else if (stat.isFile() && extensions.has(extname(path).toLowerCase())) {
      yield path;
    } else if (explicit)
      throw Error(`Expected a directory, JPEG, PNG or WebP: ${path}`);
  }
  for (const path of paths) yield* walk(path, true);
}
export async function imageCapture(
  path: string,
  instructions?: string,
): Promise<BrowserCapture> {
  const file = await open(
    path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK,
  );
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size > 20_000_000 || !stat.size)
      throw Error(`Image must be a regular file of at most 20 MB: ${path}`);
    const bytes = Buffer.alloc(stat.size + 1);
    let length = 0;
    while (length < bytes.length) {
      const part = await file.read(bytes, length, bytes.length - length, null);
      if (!part.bytesRead) break;
      length += part.bytesRead;
    }
    if (length !== stat.size)
      throw Error(`File changed while reading: ${path}`);
    const data = bytes.subarray(0, length);
    const mimeType = data.subarray(0, 3).equals(Buffer.from([255, 216, 255]))
      ? "image/jpeg"
      : data
            .subarray(0, 8)
            .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
        ? "image/png"
        : data.toString("ascii", 0, 4) === "RIFF" &&
            data.toString("ascii", 8, 12) === "WEBP"
          ? "image/webp"
          : undefined;
    if (!mimeType) throw Error(`Unsupported image content: ${path}`);
    const url =
      "gg-local:sha256:" + createHash("sha256").update(data).digest("hex");
    return validateBrowserCapture({
      version: 2,
      intent: "capture",
      url,
      title: basename(path),
      capturedAt: new Date().toISOString(),
      text: `Local image: ${basename(path)}. Preserve the supplied original image. The filename is context, not verified attribution.`,
      instructions,
      images: [
        { url, mimeType, bytes: data.toString("base64"), alt: basename(path) },
      ],
    });
  } finally {
    await file.close();
  }
}
export function importId(capture: BrowserCapture): string {
  const hash = createHash("sha256")
    .update(JSON.stringify({ ...capture, capturedAt: undefined }))
    .digest("hex");
  return `${hash.slice(0, 8)}-${hash.slice(8, 12)}-4${hash.slice(13, 16)}-a${hash.slice(17, 20)}-${hash.slice(20, 32)}`;
}
