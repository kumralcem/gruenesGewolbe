import { recordDownload } from "./archive-download.ts";
import { pipeline } from "node:stream/promises";
import { isLocalSource } from "./local-source.ts";
import type { Devices } from "./devices.ts";
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
  deviceId?: string;
  status:
    | "pending"
    | "running"
    | "completed"
    | "failed"
    | "interrupted"
    | "partial"
    | "paused"
    | "cancelled";
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
  devices?: Devices;
  publicOrigin?: string;
  usage?: () => Promise<unknown>;
  command?: (input: any, signal: AbortSignal) => Promise<unknown>;
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
  if (options.publicOrigin) {
    const u = new URL(options.publicOrigin);
    if (
      u.protocol !== "https:" ||
      u.username ||
      u.password ||
      u.pathname !== "/" ||
      u.search ||
      u.hash
    )
      throw Error("Public origin must be an HTTPS origin");
  }
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
  const token = options.devices
      ? await options.devices.pairing("capture")
      : randomBytes(32).toString("hex"),
    jobs = new Map<string, CaptureJob>(),
    queue: string[] = [];
  let active: Promise<void> | undefined,
    stopping = false,
    uploading = 0,
    activeId: string | undefined,
    current: AbortController | undefined;
  const save = async (job: CaptureJob) => {
    const dir = join(root, job.id);
    await plain(dir, true);
    const temp = join(dir, `.state-${randomUUID()}`);
    await writeFile(temp, JSON.stringify(job, null, 2), { mode: 0o600 });
    await rename(temp, join(dir, "job.json"));
  };
  const problems: string[] = [];
  let pausedUntil = 0;
  const resumeTimer = setInterval(() => {
    if (!stopping && Date.now() >= pausedUntil) {
      for (const job of jobs.values())
        if (job.status === "paused" && !queue.includes(job.id))
          queue.push(job.id);
      pump();
    }
  }, 30000);
  resumeTimer.unref();
  const pump = () => {
    if (active || stopping || !queue.length || Date.now() < pausedUntil) return;
    active = (async () => {
      while (queue.length && !stopping) {
        const id = queue.shift()!,
          job = jobs.get(id)!;
        activeId = id;
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
          if (job.status === ("cancelled" as string)) continue;
          job.result = result;
          const state = result.outcome?.status;
          job.status =
            state === "failed"
              ? "failed"
              : state === "partial"
                ? "partial"
                : state === "paused"
                  ? "paused"
                  : "completed";
          if (job.status === "paused") {
            pausedUntil = Number(result.retryAt) || Date.now() + 60000;
          }
          delete job.error;
          await save(job);
          if (
            ["saved", "existing", "upgraded", "updated", "skipped"].includes(
              result.outcome?.status ?? "",
            )
          )
            await unlink(file);
        } catch (error) {
          if (job.status !== "cancelled")
            job.status = stopping ? "interrupted" : "failed";
          job.error = error instanceof Error ? error.message : String(error);
          await save(job);
        } finally {
          current = undefined;
          activeId = undefined;
        }
        if (job.status === "paused") break;
      }
    })()
      .catch((error) => {
        // Keep pending inputs on disk when state storage itself fails.
        stopping = true;
        problems.push(`Capture processing stopped: ${String(error)}`);
      })
      .finally(() => {
        active = undefined;
        if (queue.length && !stopping && Date.now() >= pausedUntil) pump();
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
            "cancelled",
            "partial",
            "paused",
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
        if (["pending", "paused"].includes(job.status)) queue.push(id);
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
    let hasUploadSlot = false;
    try {
      const origin = req.headers.origin;
      if (
        ![
          `127.0.0.1:${(server.address() as any).port}`,
          ...(options.publicOrigin ? [new URL(options.publicOrigin).host] : []),
        ].includes(req.headers.host ?? "")
      ) {
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
      const bearer = (req.headers.authorization ?? "").replace(/^Bearer /, "");
      if (req.method === "POST" && req.url === "/pair") {
        let raw = "";
        for await (const c of req) {
          raw += c;
          if (raw.length > 1000) throw Error("Pairing request too large");
        }
        if (options.devices) {
          send(
            res,
            200,
            await options.devices.exchange(bearer, JSON.parse(raw).name),
          );
          return;
        }
        if (bearer !== token) {
          send(res, 401, { error: "Invalid pairing code" });
          return;
        }
        send(res, 200, { token });
        return;
      }
      const device = options.devices
        ? await options.devices.authenticate(bearer)
        : undefined;
      const supplied = Buffer.from(bearer),
        expected = Buffer.from(token);
      if (
        options.devices
          ? !device
          : supplied.length !== expected.length ||
            !timingSafeEqual(supplied, expected)
      ) {
        send(res, 401, { error: "Pair this device with GG" });
        return;
      }
      const canAccess = (job: CaptureJob) =>
        !options.devices ||
        device?.scope === "manage" ||
        job.deviceId === device?.id;
      const present = (job: CaptureJob) => {
        if (!options.devices || device?.scope === "manage") return job;
        return {
          id: job.id,
          status: job.status,
          url: job.url,
          title: job.title,
          createdAt: job.createdAt,
          error: job.error
            ? "Capture failed; ask a management device to inspect diagnostics."
            : undefined,
          result: job.result
            ? {
                outcome: {
                  status: job.status === "completed" ? "saved" : job.status,
                },
              }
            : undefined,
        };
      };
      const requestUrl = new URL(req.url ?? "/", "http://localhost");
      if (
        req.method === "GET" &&
        (requestUrl.pathname === "/archive" ||
          requestUrl.pathname.startsWith("/archive/"))
      ) {
        if (options.devices && device?.scope !== "manage") {
          send(res, 403, {
            error:
              "Archive browsing and downloads require management pairing. Run gg pair --scope manage, then connect with that code.",
          });
          return;
        }
        if (requestUrl.pathname === "/archive") {
          const query = (
            requestUrl.searchParams.get("q") ?? ""
          ).toLocaleLowerCase();
          const offset = Number(requestUrl.searchParams.get("offset") ?? 0);
          if (
            !Number.isSafeInteger(offset) ||
            offset < 0 ||
            query.length > 1000
          )
            throw Error("Invalid archive query");
          const records = (await options.vault.items())
            .map(({ item }) => ({
              id: item.id,
              title: item.title,
              subvault: item.subvault,
              tags: item.tags,
            }))
            .filter((item) =>
              JSON.stringify(item).toLocaleLowerCase().includes(query),
            );
          send(res, 200, {
            records: records.slice(offset, offset + 50),
            nextOffset: offset + 50 < records.length ? offset + 50 : null,
          });
          return;
        }
        const match = requestUrl.pathname.match(
          /^\/archive\/([a-f0-9-]{36})\/download$/,
        );
        if (!match) throw Error("Unknown archive route");
        const download = await recordDownload(
          options.vault,
          match[1],
          requestUrl.searchParams.get("originals") === "1",
        );
        res.writeHead(200, {
          "content-type": "application/gzip",
          "content-disposition": `attachment; filename="${download.filename}"`,
        });
        await pipeline(download.stream, res);
        return;
      }
      if (requestUrl.pathname === "/capture-rules" && req.method === "GET") {
        const subvault = requestUrl.searchParams.get("subvault") ?? "";
        send(res, 200, {
          ...(await options.vault.captureRules(subvault)),
          effective: await options.vault.effectiveCaptureRules(subvault),
          destinations: await options.vault.destinations(),
        });
        return;
      }
      if (req.url === "/capture-rules" && req.method === "POST") {
        if (options.devices && device?.scope !== "manage") {
          send(res, 403, {
            error:
              "Editing capture instructions requires management pairing. Run gg pair --scope manage and reconnect.",
          });
          return;
        }
        let raw = "";
        for await (const chunk of req) {
          raw += chunk;
          if (Buffer.byteLength(raw) > 100000)
            throw Error("Capture instructions request too large");
        }
        const input = JSON.parse(raw);
        send(
          res,
          200,
          await options.vault.setCaptureRules(
            input.text,
            input.revision,
            input.subvault ?? "",
          ),
        );
        return;
      }
      if (req.method === "GET" && req.url === "/usage") {
        send(
          res,
          200,
          options.usage
            ? await options.usage()
            : { windows: [], providers: {} },
        );
        return;
      }
      if (req.method === "POST" && req.url === "/commands") {
        if (
          !options.command ||
          (options.devices && device?.scope !== "manage")
        ) {
          send(res, 403, { error: "A management device pairing is required" });
          return;
        }
        let raw = "";
        for await (const c of req) {
          raw += c;
          if (raw.length > 200000) throw Error("Command too large");
        }
        const signal = new AbortController();
        res.once("close", () => signal.abort());
        send(res, 200, await options.command(JSON.parse(raw), signal.signal));
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
        send(
          res,
          200,
          Array.from(jobs.values())
            .filter(canAccess)
            .slice(-50)
            .reverse()
            .map(present),
        );
        return;
      }
      const selected = req.url?.match(
        /^\/captures\/([a-f0-9-]{36})(\/(?:retry|cancel))?$/,
      );
      if (selected && uuid.test(selected[1])) {
        const job = jobs.get(selected[1]);
        if (!job || !canAccess(job)) {
          send(res, 404, { error: "Unknown capture" });
          return;
        }
        if (req.method === "GET" && !selected[2]) {
          send(res, 200, present(job));
          return;
        }
        if (req.method === "POST" && selected[2] === "/cancel") {
          const cancellation = accepting.then(async () => {
            if (job.status === "completed")
              throw Error(
                "Completed captures cannot be cancelled; use undo for saved records",
              );
            job.status = "cancelled";
            for (let i = queue.length - 1; i >= 0; i--)
              if (queue[i] === job.id) queue.splice(i, 1);
            if (activeId === job.id) current?.abort();
            await save(job);
          });
          accepting = cancellation.catch(() => {});
          await cancellation;
          send(res, 200, present(job));
          return;
        }
        if (
          req.method === "POST" &&
          selected[2] === "/retry" &&
          ["failed", "interrupted", "partial", "paused", "cancelled"].includes(
            job.status,
          )
        ) {
          const retry = accepting.then(async () => {
            if (
              ![
                "failed",
                "interrupted",
                "partial",
                "paused",
                "cancelled",
              ].includes(job.status)
            )
              return;
            if (stopping || queue.length >= 20)
              throw Error("Receiver queue is full or stopping");
            await plain(join(root, job.id, "input.json"));
            job.status = "pending";
            pausedUntil = 0;
            delete job.error;
            delete job.result;
            await save(job);
            queue.push(job.id);
          });
          accepting = retry.catch(() => {});
          await retry;
          pump();
          send(res, 202, present(job));
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
      if (uploading >= 2) {
        send(res, 429, {
          error: "Two uploads are already in progress; retry shortly",
        });
        return;
      }
      uploading++;
      hasUploadSlot = true;
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
      let id = String(req.headers["x-gg-capture-id"] ?? randomUUID());
      if (!uuid.test(id)) throw Error("Invalid capture ID");
      if (device) {
        // The same client idempotency key belongs to a separate namespace per device.
        const hash = createHash("sha256")
          .update(device.id + ":" + id)
          .digest("hex");
        id = `${hash.slice(0, 8)}-${hash.slice(8, 12)}-${hash.slice(12, 16)}-${hash.slice(16, 20)}-${hash.slice(20, 32)}`;
      }
      const digest = createHash("sha256")
        .update(
          JSON.stringify(
            isLocalSource(capture.url)
              ? { ...capture, capturedAt: undefined }
              : capture,
          ),
        )
        .digest("hex");
      let accepted: CaptureJob | undefined;
      const write = accepting.then(async () => {
        if (jobs.has(id) && !canAccess(jobs.get(id)!))
          throw Error("Capture ID unavailable");
        if (isLocalSource(capture.url)) {
          const records = await options.vault.sourceRecords(capture.url);
          if (records.length) {
            accepted = {
              id,
              deviceId: device?.id,
              status: "completed",
              url: capture.url,
              title: capture.title,
              createdAt: capture.capturedAt,
              digest,
              result: {
                outcome: { status: "existing", path: records[0].path },
              },
            };
            return;
          }
          // A previously imported record may have been deleted or undone.
          if (jobs.get(id)?.status === "completed") id = randomUUID();
        }
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
          deviceId: device?.id,
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
      send(res, 202, present(accepted!));
      pump();
    } catch (error) {
      if (!res.headersSent)
        send(res, 400, {
          error: error instanceof Error ? error.message : "Capture rejected",
        });
      else res.destroy();
    } finally {
      if (hasUploadSlot) uploading--;
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
      clearInterval(resumeTimer);
      current?.abort();
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
      await accepting;
      if (active) await active;
      await unlink(lock);
    },
  };
}
