import http from "node:http";
import {
  randomBytes,
  randomUUID,
  createHash,
  timingSafeEqual,
} from "node:crypto";
import {
  mkdir,
  readFile,
  writeFile,
  rename,
  readdir,
  lstat,
  unlink,
  rm,
} from "node:fs/promises";
import { join } from "node:path";
import type { Vault } from "./vault.ts";
import {
  validateBrowserCapture,
  type BrowserCapture,
} from "./browser-capture.ts";

interface CaptureJob {
  id: string;
  status: "pending" | "running" | "completed" | "failed" | "interrupted";
  url: string;
  title: string;
  createdAt: string;
  digest: string;
  result?: unknown;
  error?: string;
}
export interface ReceiverOptions {
  vault: Vault;
  port?: number;
  processCapture: (
    capture: BrowserCapture,
    signal: AbortSignal,
  ) => Promise<{ outcome?: { status: string }; [key: string]: unknown }>;
}
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
async function plain(path: string, dir = false) {
  const stat = await lstat(path);
  if (stat.isSymbolicLink() || (dir ? !stat.isDirectory() : !stat.isFile()))
    throw Error("Unsafe receiver state entry");
}
export async function createReceiver(options: ReceiverOptions) {
  const root = join(options.vault.root, ".gg-jobs");
  await mkdir(root, { recursive: true, mode: 0o700 });
  await plain(root, true);
  const lock = join(root, ".receiver-lock");
  await writeFile(lock, String(process.pid), { flag: "wx", mode: 0o600 }).catch(
    () => {
      throw Error(
        "A receiver is running, or .gg-jobs/.receiver-lock needs checking after a crash",
      );
    },
  );
  const token = randomBytes(32).toString("hex"),
    jobs = new Map<string, CaptureJob>(),
    queue: string[] = [];
  let active: Promise<void> | undefined,
    stopping = false,
    current: AbortController | undefined;
  const save = async (job: CaptureJob) => {
    const dir = join(root, job.id);
    await plain(dir, true);
    const temp = join(dir, `.state-${randomUUID()}`);
    await writeFile(temp, JSON.stringify(job, null, 2), { mode: 0o600 });
    await rename(temp, join(dir, "job.json"));
  };
  const problems: string[] = [];
  const pump = () => {
    if (active || stopping) return;
    active = (async () => {
      while (queue.length && !stopping) {
        const id = queue.shift()!,
          job = jobs.get(id)!;
        current = new AbortController();
        try {
          job.status = "running";
          await save(job);
          const file = join(root, id, "input.json");
          await plain(file);
          const capture = validateBrowserCapture(
            JSON.parse(await readFile(file, "utf8")),
          );
          const result = await options.processCapture(capture, current.signal);
          job.result = result;
          job.status =
            result.outcome?.status === "failed" ? "failed" : "completed";
          await save(job);
          if (
            ["saved", "existing", "upgraded", "skipped"].includes(
              result.outcome?.status ?? "",
            )
          )
            await unlink(file);
        } catch (error) {
          job.status = stopping ? "interrupted" : "failed";
          job.error = error instanceof Error ? error.message : String(error);
          await save(job);
        } finally {
          current = undefined;
        }
      }
    })()
      .catch((error) => {
        // Keep pending inputs on disk when state storage itself fails.
        stopping = true;
        problems.push(`Capture processing stopped: ${String(error)}`);
      })
      .finally(() => {
        active = undefined;
        if (queue.length && !stopping) pump();
      });
  };
  try {
    for (const id of await readdir(root)) {
      if (!uuid.test(id)) continue;
      try {
        await plain(join(root, id), true);
        await plain(join(root, id, "job.json"));
        const job = JSON.parse(
          await readFile(join(root, id, "job.json"), "utf8"),
        ) as CaptureJob;
        if (
          job.id !== id ||
          ![
            "pending",
            "running",
            "completed",
            "failed",
            "interrupted",
          ].includes(job.status)
        )
          throw Error("Invalid stored job");
        if (job.status === "running") {
          job.status = "interrupted";
          job.error =
            "The previous receiver stopped during processing; retry explicitly";
          await save(job);
        }
        jobs.set(id, job);
        if (job.status === "pending") queue.push(id);
      } catch (error) {
        problems.push(`Could not load capture ${id}: ${String(error)}`);
      }
    }
  } catch (error) {
    await unlink(lock);
    throw error;
  }
  let accepting = Promise.resolve();
  const send = (res: http.ServerResponse, status: number, data: unknown) => {
    res.writeHead(status, {
      "content-type": "application/json",
      "cache-control": "no-store",
    });
    res.end(JSON.stringify(data));
  };
  const server = http.createServer(async (req, res) => {
    try {
      const origin = req.headers.origin;
      if (req.headers.host !== `127.0.0.1:${(server.address() as any).port}`) {
        send(res, 403, { error: "Invalid receiver host" });
        return;
      }
      if (origin && !/^chrome-extension:\/\/[a-p]{32}$/.test(origin)) {
        send(res, 403, {
          error: "Only a paired extension may access this receiver",
        });
        return;
      }
      if (origin) {
        res.setHeader("access-control-allow-origin", origin);
        res.setHeader("vary", "Origin");
      }
      if (req.method === "OPTIONS") {
        res.setHeader("access-control-allow-methods", "GET,POST");
        res.setHeader(
          "access-control-allow-headers",
          "Authorization,Content-Type,X-GG-Capture-ID",
        );
        res.writeHead(204);
        res.end();
        return;
      }
      const supplied = Buffer.from(req.headers.authorization ?? "");
      const expected = Buffer.from(`Bearer ${token}`);
      if (
        supplied.length !== expected.length ||
        !timingSafeEqual(supplied, expected)
      ) {
        send(res, 401, { error: "Pair the extension with the receiver token" });
        return;
      }
      if (req.method === "GET" && req.url === "/health") {
        send(res, 200, {
          version: 1,
          subvaults: options.vault.areas,
          problems,
        });
        return;
      }
      if (req.method === "GET" && req.url === "/captures") {
        send(res, 200, Array.from(jobs.values()).slice(-50).reverse());
        return;
      }
      const selected = req.url?.match(
        /^\/captures\/([a-f0-9-]{36})(\/retry)?$/,
      );
      if (selected && uuid.test(selected[1])) {
        const job = jobs.get(selected[1]);
        if (!job) {
          send(res, 404, { error: "Unknown capture" });
          return;
        }
        if (req.method === "GET" && !selected[2]) {
          send(res, 200, job);
          return;
        }
        if (
          req.method === "POST" &&
          selected[2] &&
          ["failed", "interrupted"].includes(job.status)
        ) {
          const retry = accepting.then(async () => {
            if (!["failed", "interrupted"].includes(job.status)) return;
            if (stopping || queue.length >= 20)
              throw Error("Receiver queue is full or stopping");
            await plain(join(root, job.id, "input.json"));
            job.status = "pending";
            delete job.error;
            delete job.result;
            await save(job);
            queue.push(job.id);
          });
          accepting = retry.catch(() => {});
          await retry;
          pump();
          send(res, 202, job);
          return;
        }
      }
      if (req.method !== "POST" || req.url !== "/captures") {
        send(res, 404, { error: "Unknown receiver operation" });
        return;
      }
      if (!req.headers["content-type"]?.startsWith("application/json"))
        throw Error("JSON capture required");
      if (stopping || queue.length >= 20)
        throw Error("Receiver queue is full or stopping");
      let size = 0;
      const chunks: Buffer[] = [];
      for await (const chunk of req) {
        size += chunk.length;
        if (size > 90000000) throw Error("Browser capture exceeds 90 MB");
        chunks.push(chunk);
      }
      const capture = validateBrowserCapture(
        JSON.parse(Buffer.concat(chunks).toString()),
      );
      const id = String(req.headers["x-gg-capture-id"] ?? randomUUID());
      if (!uuid.test(id)) throw Error("Invalid capture ID");
      const digest = createHash("sha256")
        .update(JSON.stringify(capture))
        .digest("hex");
      let accepted: CaptureJob | undefined;
      const write = accepting.then(async () => {
        const old = jobs.get(id);
        if (old) {
          if (old.digest !== digest)
            throw Error("Capture ID was already used for different content");
          accepted = old;
          return;
        }
        if (stopping || queue.length >= 20)
          throw Error("Receiver queue is full or stopping");
        let retained = 0;
        for (const entry of jobs.values()) {
          try {
            retained += (await lstat(join(root, entry.id, "input.json"))).size;
          } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
          }
        }
        if (retained + Buffer.byteLength(JSON.stringify(capture)) > 256000000)
          throw Error(
            "Retained captures exceed 256 MB; resolve or remove old failed capture inputs first",
          );
        const job: CaptureJob = {
          id,
          status: "pending",
          url: capture.url,
          title: capture.title,
          createdAt: new Date().toISOString(),
          digest,
        };
        const stage = join(root, `.incoming-${randomUUID()}`);
        await mkdir(stage, { mode: 0o700 });
        try {
          await writeFile(join(stage, "input.json"), JSON.stringify(capture), {
            flag: "wx",
            mode: 0o600,
          });
          await writeFile(
            join(stage, "job.json"),
            JSON.stringify(job, null, 2),
            { flag: "wx", mode: 0o600 },
          );
          await rename(stage, join(root, id));
        } finally {
          await rm(stage, { recursive: true, force: true });
        }
        jobs.set(id, job);
        queue.push(id);
        accepted = job;
      });
      accepting = write.catch(() => {});
      await write;
      send(res, 202, accepted);
      pump();
    } catch (error) {
      if (!res.headersSent)
        send(res, 400, {
          error: error instanceof Error ? error.message : "Capture rejected",
        });
      else res.destroy();
    }
  });
  server.requestTimeout = 30000;
  server.headersTimeout = 10000;
  try {
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(options.port ?? 48123, "127.0.0.1", resolve);
    });
  } catch (error) {
    await unlink(lock);
    throw error;
  }
  pump();
  return {
    url: `http://127.0.0.1:${(server.address() as any).port}`,
    token,
    problems,
    async idle() {
      await accepting;
      while (active) await active;
    },
    async close() {
      stopping = true;
      current?.abort();
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
      await accepting;
      if (active) await active;
      await unlink(lock);
    },
  };
}
