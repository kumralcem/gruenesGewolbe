import { createHash, randomUUID } from "node:crypto";
import {
  mkdir,
  lstat,
  readFile,
  readdir,
  writeFile,
  rename,
  unlink,
  rmdir,
} from "node:fs/promises";
import { join, dirname, relative, resolve } from "node:path";

type Snapshot = Record<string, string>;
export interface Change {
  id: string;
  label: string;
  at: string;
  batch?: string;
  before: Snapshot;
  after: Snapshot;
  roots: string[];
  undone?: string;
  incomplete?: boolean;
}
const hash = (b: Buffer) => createHash("sha256").update(b).digest("hex");
export class History {
  constructor(private root: string) {}
  private get dir() {
    return join(this.root, ".gg-history");
  }
  private async safe(path: string) {
    const full = resolve(this.root, path);
    if (!full.startsWith(this.root + "/") || path.split("/").includes(".."))
      throw Error("Unsafe history path");
    let current = this.root;
    for (const part of relative(this.root, full).split("/")) {
      current = join(current, part);
      try {
        if ((await lstat(current)).isSymbolicLink())
          throw Error("Unsafe symlink in history path");
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
      }
    }
    return full;
  }
  private async snapshot(roots: string[], store: boolean): Promise<Snapshot> {
    const result: Snapshot = {};
    if (store)
      await mkdir(await this.safe(".gg-history/blobs"), {
        recursive: true,
        mode: 0o700,
      });
    const visit = async (path: string) => {
      const full = await this.safe(path);
      const stat = await lstat(full).catch((e: NodeJS.ErrnoException) => {
        if (e.code !== "ENOENT") throw e;
      });
      if (!stat) return;
      if (stat.isDirectory()) {
        result[path] = "directory";
        for (const child of await readdir(full))
          await visit(path + "/" + child);
      } else if (stat.isFile()) {
        const bytes = await readFile(full),
          digest = hash(bytes);
        result[path] = digest;
        if (store)
          await writeFile(
            await this.safe(".gg-history/blobs/" + digest),
            bytes,
            { flag: "wx", mode: 0o600 },
          ).catch((e: NodeJS.ErrnoException) => {
            if (e.code !== "EEXIST") throw e;
          });
      } else throw Error("Unsupported history entry");
    };
    for (const path of roots) await visit(path);
    return result;
  }
  private async write(change: Change, pending = false) {
    const path = await this.safe(
      `.gg-history/${change.id}${pending ? ".pending" : ".json"}`,
    );
    const temp = path + "." + randomUUID();
    await writeFile(temp, JSON.stringify(change), { mode: 0o600 });
    await rename(temp, path);
  }
  async run<T>(
    label: string,
    roots: string[],
    action: () => Promise<T>,
    batch?: string,
  ): Promise<{ value: T; operationId: string }> {
    const before = await this.snapshot(roots, true);
    const change: Change = {
      id: randomUUID(),
      label,
      at: new Date().toISOString(),
      batch,
      roots,
      before,
      after: {},
    };
    await this.write(change, true);
    try {
      const value = await action();
      change.after = await this.snapshot(roots, true);
      await this.write(change);
      await unlink(join(this.dir, change.id + ".pending"));
      return { value, operationId: change.id };
    } catch (e) {
      // Preserve even partial changes so recovery remains explicit and reversible.
      change.after = await this.snapshot(roots, true);
      change.label += " (interrupted)";
      await this.write(change);
      await unlink(join(this.dir, change.id + ".pending"));
      throw e;
    }
  }
  async fingerprint(roots: string[]) {
    const snapshot = await this.snapshot(roots, false);
    return hash(
      Buffer.from(
        JSON.stringify(
          Object.entries(snapshot).sort(([a], [b]) => a.localeCompare(b)),
        ),
      ),
    );
  }
  async list(): Promise<Change[]> {
    const files: string[] = await readdir(this.dir).catch(
      (e: NodeJS.ErrnoException) => {
        if (e.code === "ENOENT") return [];
        throw e;
      },
    );
    return (
      await Promise.all(
        files
          .filter(
            (f) =>
              /^[a-f0-9-]{36}\.(json|pending)$/.test(f) &&
              !(
                f.endsWith(".pending") &&
                files.includes(f.replace(".pending", ".json"))
              ),
          )
          .map(
            async (f) =>
              ({
                ...JSON.parse(
                  await readFile(await this.safe(".gg-history/" + f), "utf8"),
                ),
                ...(f.endsWith(".pending")
                  ? {
                      incomplete: true,
                      label: "Interrupted operation; manual recovery required",
                    }
                  : {}),
              }) as Change,
          ),
      )
    ).sort((a, b) => b.at.localeCompare(a.at));
  }
  async undo(id?: string): Promise<string[]> {
    const changes = await this.list();
    const selected = id
      ? changes.filter((c) => c.id === id || c.batch === id)
      : changes.filter((c) => !c.undone).slice(0, 1);
    if (!selected.length) throw Error("No matching operation to undo");
    // Preflight the whole batch against a simulated filesystem before any writes.
    const roots = [...new Set(selected.flatMap((c) => c.roots))];
    const state = await this.snapshot(roots, false);
    for (const c of selected) {
      if (c.incomplete)
        throw Error(
          "Interrupted history operation needs manual recovery before undo",
        );
      if (c.undone) throw Error("Operation already undone");
      if (
        /^(create-subvault|rename-subvault)( \(interrupted\))?$/.test(
          c.label,
        ) &&
        ![...Object.values(c.before), ...Object.values(c.after)].includes(
          "directory",
        )
      )
        throw Error(
          "Older folder history lacks directory information; manual recovery required",
        );
      for (const [path, kind] of Object.entries(c.after)) {
        if (kind !== "directory" || c.before[path]) continue;
        for (const entry of Object.keys(state))
          if (entry.startsWith(path + "/") && !(entry in c.after))
            throw Error(`Undo conflicts with later additions: ${entry}`);
      }
      for (const p of new Set([
        ...Object.keys(c.before),
        ...Object.keys(c.after),
      ])) {
        if (c.before[p] === c.after[p]) continue;
        if (state[p] !== c.after[p])
          throw Error(`Undo conflicts with later edits: ${p}`);
        if (c.before[p]) state[p] = c.before[p];
        else delete state[p];
      }
    }
    // Verify every restore blob before changing any live file.
    for (const digest of new Set(
      selected.flatMap((c) => Object.values(c.before)),
    )) {
      if (digest === "directory") continue;
      if (
        !/^[a-f0-9]{64}$/.test(digest) ||
        hash(await readFile(await this.safe(".gg-history/blobs/" + digest))) !==
          digest
      )
        throw Error("History blob integrity check failed");
    }
    for (const c of selected) {
      for (const p of [
        ...new Set([...Object.keys(c.before), ...Object.keys(c.after)]),
      ].sort((a, b) => b.split("/").length - a.split("/").length)) {
        if (c.before[p] === c.after[p]) continue;
        const full = await this.safe(p);
        if (c.before[p] === "directory") {
          await mkdir(full, { recursive: true });
          continue;
        }
        if (!c.before[p] && c.after[p] === "directory") {
          await rmdir(full).catch((e: NodeJS.ErrnoException) => {
            if (e.code !== "ENOENT") throw e;
          });
          continue;
        }
        if (c.before[p]) {
          const bytes = await readFile(
            await this.safe(".gg-history/blobs/" + c.before[p]),
          );
          if (hash(bytes) !== c.before[p])
            throw Error("History blob integrity check failed");
          await mkdir(dirname(full), { recursive: true });
          const temp = full + ".restore-" + randomUUID();
          await writeFile(temp, bytes, { mode: 0o600 });
          await rename(temp, full);
        } else {
          await unlink(full);
          let dir = dirname(full);
          const scope = c.roots
            .filter((r) => p === r || p.startsWith(r + "/"))
            .sort((a, b) => b.length - a.length)[0];
          const stop = dirname(join(this.root, scope));
          while (dir !== this.root && dir !== stop) {
            if (c.before[relative(this.root, dir)] === "directory") break;
            try {
              await rmdir(dir);
            } catch {
              break;
            }
            dir = dirname(dir);
          }
        }
      }
      c.undone = new Date().toISOString();
      await this.write(c);
    }
    return selected.map((c) => c.id);
  }
}
