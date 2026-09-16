import {
  mkdir,
  readFile,
  writeFile,
  rename,
  unlink,
  lstat,
} from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { dirname, join } from "node:path";
import { homedir } from "node:os";
import { setTimeout as delay } from "node:timers/promises";
export const defaultStateDir = () =>
  process.env.GG_STATE_DIR || join(homedir(), ".config", "gg");
export async function privateDirectory(dir: string) {
  await mkdir(dir, { recursive: true, mode: 0o700 });
  const stat = await lstat(dir);
  if (!stat.isDirectory() || stat.isSymbolicLink())
    throw Error("Unsafe state directory");
}
export async function readJson<T>(path: string, fallback: T): Promise<T> {
  try {
    const stat = await lstat(path);
    if (!stat.isFile() || stat.isSymbolicLink())
      throw Error("Unsafe state file");
    return JSON.parse(await readFile(path, "utf8"));
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return fallback;
    throw e;
  }
}
export async function writeJson(path: string, value: unknown) {
  await privateDirectory(dirname(path));
  const tmp = path + "." + randomUUID();
  await writeFile(tmp, JSON.stringify(value, null, 2), {
    mode: 0o600,
    flag: "wx",
  });
  await rename(tmp, path);
}
export async function fileLock<T>(
  path: string,
  action: () => Promise<T>,
  waitMs = 2000,
  signal?: AbortSignal,
): Promise<T> {
  await privateDirectory(dirname(path));
  const started = Date.now();
  while (true) {
    signal?.throwIfAborted();
    try {
      await writeFile(path, String(process.pid), { flag: "wx", mode: 0o600 });
      break;
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
      const dead = async () => {
        try {
          const stat = await lstat(path);
          if (stat.isSymbolicLink() || !stat.isFile())
            throw Error("Unsafe controller lock");
          const pid = Number(await readFile(path, "utf8"));
          // A contender can observe the interval between creation and writing.
          if (!Number.isSafeInteger(pid) || pid < 1) return false;
          process.kill(pid, 0);
          return false;
        } catch (error) {
          const code = (error as NodeJS.ErrnoException).code;
          if (code === "ESRCH") return true;
          if (code === "EPERM" || code === "ENOENT") return false;
          throw error;
        }
      };
      if (await dead()) {
        // Serialize reclamation and re-read the owner. Never unlink a lock that
        // another contender installed after our original dead-owner check.
        let reclaim = false;
        try {
          await writeFile(path + ".reclaim", String(process.pid), {
            flag: "wx",
            mode: 0o600,
          });
          reclaim = true;
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
        }
        if (reclaim) {
          try {
            if (await dead())
              await unlink(path).catch((error) => {
                if (error.code !== "ENOENT") throw error;
              });
          } finally {
            await unlink(path + ".reclaim");
          }
          continue;
        }
      }
      if (Date.now() - started >= waitMs)
        throw Error("GG is busy; another operation holds the controller lock");
      await delay(25, undefined, { signal });
    }
  }
  try {
    return await action();
  } finally {
    await unlink(path);
  }
}
