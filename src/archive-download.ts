import { originalTextFile } from "./text-import.ts";
import { constants } from "node:fs";
import { open, lstat } from "node:fs/promises";
import { join, relative, sep } from "node:path";
import { Readable } from "node:stream";
import { createGzip } from "node:zlib";
import type { Vault } from "./vault.ts";

async function safeFile(root: string, path: string) {
  const parts = relative(root, path).split(sep);
  if (parts.some((p) => !p || p === ".." || p.startsWith(".")))
    throw Error("Unsafe export path");
  let current = root;
  for (const part of parts.slice(0, -1)) {
    current = join(current, part);
    const stat = await lstat(current);
    if (!stat.isDirectory() || stat.isSymbolicLink())
      throw Error("Unsafe export directory");
  }
  const handle = await open(
    path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK,
  );
  const stat = await handle.stat();
  if (!stat.isFile() || stat.size > 64_000_000) {
    await handle.close();
    throw Error("Unsupported export file");
  }
  return { handle, stat };
}
function header(name: string, size: number) {
  if (Buffer.byteLength(name) > 100) throw Error("Export filename too long");
  const block = Buffer.alloc(512);
  block.write(name, 0, 100);
  const octal = (offset: number, width: number, value: number) =>
    block.write(
      value.toString(8).padStart(width - 1, "0") + "\0",
      offset,
      width,
    );
  octal(100, 8, 0o600);
  octal(108, 8, 0);
  octal(116, 8, 0);
  octal(124, 12, size);
  octal(136, 12, 0);
  block.fill(32, 148, 156);
  block.write("0", 156);
  block.write("ustar\0", 257);
  block.write("00", 263);
  octal(
    148,
    8,
    block.reduce((sum, byte) => sum + byte, 0),
  );
  return block;
}

/** Only public record artifacts, never controller state, history or policy files. */
export async function recordDownload(
  vault: Vault,
  id: string,
  originalsOnly = false,
) {
  const record = (await vault.items()).find((r) => r.item.id === id);
  if (!record) throw Error("Unknown record ID");
  const names = new Set<string>(originalsOnly ? [] : ["record.md"]);
  if (record.item.originalFile) {
    const file = record.item.originalFile;
    if (file !== originalTextFile(file.slice(9)))
      throw Error("Unsafe original document path");
    names.add(file);
  }
  for (const asset of record.item.assets) {
    if (!/^files\/[a-f0-9]{64}\.(jpg|png|webp)$/.test(asset.file))
      throw Error("Unsafe preserved asset path");
    names.add(asset.file);
  }
  if (!originalsOnly)
    for (const name of ["source.md", "source.txt", "preview.jpg"]) {
      try {
        await lstat(join(record.path, name));
        names.add(name);
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
      }
    }
  if (!names.size) throw Error("This record has no original files");
  // Open before responding: validates every entry and pins the file version even
  // if an archive operation renames/replaces it while the download streams.
  const files: {
    name: string;
    handle: Awaited<ReturnType<typeof open>>;
    size: number;
  }[] = [];
  try {
    let total = 0;
    for (const name of names) {
      const { handle, stat } = await safeFile(
        vault.root,
        join(record.path, name),
      );
      files.push({ name, handle, size: stat.size });
      total += stat.size;
      if (total > 128_000_000)
        throw Error("Record bundle exceeds the current 128 MB download limit");
    }
  } catch (e) {
    await Promise.all(files.map((f) => f.handle.close()));
    throw e;
  }
  async function* tar() {
    try {
      for (const file of files) {
        yield header(file.name, file.size);
        let offset = 0;
        while (offset < file.size) {
          const buffer = Buffer.alloc(Math.min(65536, file.size - offset));
          const { bytesRead } = await file.handle.read(
            buffer,
            0,
            buffer.length,
            offset,
          );
          if (!bytesRead) throw Error("File changed during export");
          offset += bytesRead;
          yield buffer.subarray(0, bytesRead);
        }
        if (file.size % 512) yield Buffer.alloc(512 - (file.size % 512));
      }
      yield Buffer.alloc(1024);
    } finally {
      await Promise.all(files.map((f) => f.handle.close()));
    }
  }
  const source = Readable.from(tar());
  const stream = createGzip();
  source.on("error", (error) => stream.destroy(error));
  stream.on("close", () => source.destroy());
  source.pipe(stream);
  return {
    stream,
    filename: `${id}${originalsOnly ? "-originals" : ""}.tar.gz`,
  };
}
