import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { Duplex } from "node:stream";
import { createGateway, type GatewayOptions } from "./gateway.ts";
import type { Job } from "./types.ts";
import { Channel } from "./ipc.ts";

export class JobCancelledError extends Error {}

// Only the controller owns the vault, provider key and runtime. FD 3 connects
// the worker to this job's gateway; it conveys no selectable host destination.
export async function runJob(
  options: GatewayOptions & { focus?: string; onEvent?: (event: any) => void },
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
    (stopping ??= new Promise<void>((resolve) => {
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
      cleanup.once("error", done);
      cleanup.once("exit", done);
    }));
  const onDeadline = () => {
    void stop();
  };
  const interrupted = () => {
    cancelled = true;
    void stop();
  };
  gateway.signal.addEventListener("abort", onDeadline);
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
  const { apiKey: _secret, ...config } = options.config;
  const job: Job = {
    intent: options.intent,
    input: options.input,
    focus: options.focus,
    config,
    token: gateway.token,
    areas: options.vault.areas,
    fixtures: Boolean(options.fixturePage),
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
    const outcome =
      gateway.outcomes[0] ??
      (options.intent === "ask" && events.some((e) => e.type === "results")
        ? undefined
        : options.intent === "probe" && exitCode === 0
          ? undefined
          : { status: "failed", reason, sourceUrl: options.input });
    return {
      input: options.input,
      exitCode,
      outcome,
      answers: gateway.answers,
      events,
      modelRequests: gateway.requests,
      gatewayEvents: gateway.events,
      stderr,
    };
  } finally {
    process.removeListener("SIGINT", interrupted);
    process.removeListener("SIGTERM", interrupted);
    gateway.signal.removeEventListener("abort", onDeadline);
    await stop();
    channel.close();
    await gateway.close();
  }
}
