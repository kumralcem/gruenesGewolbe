import { fileLock, defaultStateDir } from "./state.ts";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { Duplex } from "node:stream";
import { createGateway, type GatewayOptions } from "./gateway.ts";
import type { Job } from "./types.ts";
import { Channel } from "./ipc.ts";

export class JobCancelledError extends Error {}

// Only the controller owns the vault, provider key and runtime. FD 3 connects
// the worker to this job's gateway; it conveys no selectable host destination.
async function runWorker(
  options: GatewayOptions & {
    focus?: string;
    onEvent?: (event: any) => void;
    signal?: AbortSignal;
  },
) {
  const gateway = await createGateway(options);
  const name = `gg-prototype-${randomUUID()}`;
  const events: any[] = [];
  let stderr = "",
    pending = "",
    outputBytes = 0,
    cancelled = false,
    outputExceeded = false;
  const args = [
    "run",
    "--rm",
    "-i",
    "--name",
    name,
    "--network=none",
    "--read-only",
    "--cap-drop=ALL",
    "--cap-add=SYS_CHROOT",
    "--security-opt=no-new-privileges",
    "--userns=keep-id",
    `--user=${process.getuid?.() ?? 1000}:${process.getgid?.() ?? 1000}`,
    "--memory=2g",
    "--cpus=2",
    "--pids-limit=256",
    "--tmpfs=/work:rw,size=768m,mode=1777",
    "--tmpfs=/tmp:rw,size=256m,mode=1777",
    "--shm-size=256m",
    "--preserve-fds=1",
    "localhost/gg-pi-prototype",
  ];
  const runtimeEnv = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR,
  };
  const child = spawn("podman", args, {
    stdio: ["pipe", "pipe", "pipe", "pipe"],
    env: runtimeEnv,
  });
  const channel = new Channel(child.stdio[3] as Duplex, gateway.socket);
  let stopping: Promise<void> | undefined;
  const stop = () =>
    child.pid === undefined
      ? Promise.resolve()
      : (stopping ??= new Promise<void>((resolve) => {
          const cleanup = spawn("podman", ["rm", "--force", name], {
            stdio: "ignore",
            env: runtimeEnv,
          });
          const timer = setTimeout(() => {
            cleanup.kill("SIGKILL");
            child.kill("SIGKILL");
          }, 10000);
          const done = () => {
            clearTimeout(timer);
            resolve();
          };
          const failedRemoval = () => {
            // Creation may have raced the first lookup. Retry after the launcher
            // has stopped, rather than memoizing a failed "not found" removal.
            const retryRemoval = () => {
              const retry = spawn("podman", ["rm", "--force", name], {
                stdio: "ignore",
                env: runtimeEnv,
              });
              const retryTimer = setTimeout(() => retry.kill("SIGKILL"), 5000);
              const finished = () => {
                clearTimeout(retryTimer);
                done();
              };
              retry.once("error", finished);
              retry.once("exit", finished);
            };
            if (child.exitCode !== null || child.signalCode !== null)
              retryRemoval();
            else {
              child.once("exit", retryRemoval);
              child.kill("SIGKILL");
            }
          };
          cleanup.once("error", failedRemoval);
          cleanup.once("exit", (code) =>
            code === 0 ? done() : failedRemoval(),
          );
        }));
  const onDeadline = () => {
    void stop();
  };
  const interrupted = () => {
    cancelled = true;
    void stop();
  };
  gateway.signal.addEventListener("abort", onDeadline);
  options.signal?.addEventListener("abort", interrupted, { once: true });
  if (options.signal?.aborted) interrupted();
  process.once("SIGINT", interrupted);
  process.once("SIGTERM", interrupted);
  child.stdout.on("data", (chunk: Buffer) => {
    outputBytes += chunk.length;
    if (outputBytes > 4_000_000) {
      outputExceeded = true;
      void stop();
      return;
    }
    pending += chunk.toString();
    let at: number;
    while ((at = pending.indexOf("\n")) >= 0) {
      const line = pending.slice(0, at);
      pending = pending.slice(at + 1);
      try {
        const event = JSON.parse(line);
        events.push(event);
        options.onEvent?.(event);
      } catch {
        /* Ignore non-protocol package output. */
      }
    }
  });
  child.stderr.on("data", (chunk: Buffer) => {
    stderr = (stderr + chunk.toString()).slice(-16000);
  });
  const {
    provider,
    model,
    api,
    vision,
    maxOutputTokens,
    maxSeconds,
    maxRequests,
  } = options.config;
  const config = {
    provider,
    model,
    api,
    vision,
    maxOutputTokens,
    maxSeconds,
    maxRequests,
  };
  const job: Job = {
    intent: options.intent,
    input: options.input,
    focus: options.focus,
    config,
    token: gateway.token,
    areas: options.vault.areas,
    fixtures: Boolean(options.fixturePage),
    browserCapture: Boolean(options.browserCapture),
  };
  child.stdin.on("error", () => {});
  child.stdin.end(JSON.stringify(job));
  try {
    const exitCode = await new Promise<number>((resolve, reject) => {
      child.once("error", reject);
      child.once("exit", (code) => resolve(code ?? 1));
    });
    if (cancelled)
      throw new JobCancelledError(
        "Cancelled; remaining batch entries were not started",
      );
    const reason = outputExceeded
      ? "Worker output budget exceeded"
      : gateway.signal.aborted
        ? "Job deadline exceeded"
        : (events.findLast((e) => e.type === "error")?.message ??
          `Worker exited without an outcome (${exitCode})`);
    const omissions = [
      ...(options.browserCapture?.warnings ?? []),
      ...(options.browserCapture?.images ?? [])
        .filter((i) => !i.bytes)
        .map((i) => i.error ?? "Image unavailable"),
    ];
    const captureWarning =
      omissions.length || gateway.outcomes.some((o) => o.status === "partial")
        ? "Saved available content; some page content or images were unavailable. Open the original page and recapture to supply missing material."
        : gateway.outcomes.some((o) => o.conflicts?.length)
          ? "Saved update while preserving conflicting manual edits; review the dated record note."
          : undefined;
    const outcome =
      (options.intent === "capture"
        ? {
            status: gateway.paused
              ? "paused"
              : gateway.outcomes.some((o) => o.status === "skipped") &&
                  exitCode === 0
                ? "skipped"
                : gateway.complete && exitCode === 0 && captureWarning
                  ? "partial"
                  : gateway.complete && exitCode === 0
                    ? gateway.outcomes.some((o) => o.status === "updated")
                      ? "updated"
                      : "saved"
                    : gateway.outcomes.length
                      ? "partial"
                      : "failed",
            reason:
              gateway.paused ??
              captureWarning ??
              gateway.outcomes.find((o) => o.status === "skipped")?.reason ??
              (gateway.complete && exitCode === 0 ? undefined : reason),
            sourceUrl: options.input,
            path: gateway.outcomes[0]?.path,
          }
        : options.intent === "manage" && exitCode === 0
          ? undefined
          : gateway.outcomes[0]) ??
      ((options.intent === "manage" && exitCode === 0) ||
      (options.intent === "ask" && events.some((e) => e.type === "results"))
        ? undefined
        : options.intent === "probe" && exitCode === 0
          ? undefined
          : { status: "failed", reason, sourceUrl: options.input });
    return {
      input: options.input,
      exitCode,
      outcome,
      answers: gateway.answers,
      outcomes: gateway.outcomes,
      management: gateway.management,
      batchId: gateway.batchId,
      retryAt: gateway.retryAt,
      answer: events.findLast((e) => e.type === "answer")?.text,
      events,
      modelRequests: gateway.requests,
      gatewayEvents: gateway.events,
      vaultProblems: Array.from(options.vault.problems, ([path, reason]) => ({
        path,
        reason,
      })),
      stderr,
    };
  } finally {
    options.signal?.removeEventListener("abort", interrupted);
    process.removeListener("SIGINT", interrupted);
    process.removeListener("SIGTERM", interrupted);
    gateway.signal.removeEventListener("abort", onDeadline);
    await stop();
    channel.close();
    await gateway.close();
  }
}

export async function runJob(options: Parameters<typeof runWorker>[0]) {
  const stateDir =
    options.config.stateDir ??
    (options.mockModel
      ? join(options.vault.root, ".fixture-controller")
      : defaultStateDir());
  return fileLock(
    join(stateDir, "job.lock"),
    () => runWorker({ ...options, config: { ...options.config, stateDir } }),
    600000,
    options.signal,
  );
}
