import { join } from "node:path";
import { readJson, writeJson } from "./state.ts";
import type { Command } from "./controller.ts";
export interface Connection {
  url: string;
  token: string;
  id?: string;
}
export function endpoint(value: string) {
  const url = new URL(value);
  if (
    (url.protocol !== "https:" &&
      !(url.protocol === "http:" && url.hostname === "127.0.0.1")) ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.hash ||
    url.search
  )
    throw Error("Use an HTTPS server origin or local http://127.0.0.1");
  return url.origin;
}
export async function connect(dir: string, url: string, code: string) {
  url = endpoint(url);
  const response = await fetch(url + "/pair", {
    method: "POST",
    redirect: "error",
    headers: {
      authorization: `Bearer ${code}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({ name: "GG CLI" }),
    signal: AbortSignal.timeout(30000),
  });
  const result = (await response.json()) as any;
  if (!response.ok) throw Error(result.error ?? "Pairing failed");
  await writeJson(join(dir, "client.json"), {
    url,
    token: result.token,
    id: result.id,
  });
  return { url, id: result.id, scope: result.scope };
}
export async function connection(dir: string) {
  return readJson<Connection | undefined>(join(dir, "client.json"), undefined);
}
export async function remote(connection: Connection, command: Command) {
  const response = await fetch(endpoint(connection.url) + "/commands", {
    method: "POST",
    redirect: "error",
    headers: {
      authorization: `Bearer ${connection.token}`,
      "content-type": "application/json",
    },
    body: JSON.stringify(command),
    signal: AbortSignal.timeout(660000),
  });
  const result = (await response.json()) as any;
  if (!response.ok) throw Error(result.error ?? "Remote command failed");
  return result;
}

/** Submit one image through the same durable queue as browser captures. */
export async function importRemote(
  connection: Connection,
  capture: import("./browser-capture.ts").BrowserCapture,
  id: string,
  report: (status: string) => void = () => {},
) {
  const request = async (path: string, method = "GET", body?: unknown) => {
    const response = await fetch(endpoint(connection.url) + path, {
      method,
      redirect: "error",
      headers: {
        authorization: `Bearer ${connection.token}`,
        "content-type": "application/json",
        "x-gg-capture-id": id,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(60000),
    });
    const value = (await response.json()) as any;
    if (!response.ok)
      throw Error(value.error ?? `Import failed (${response.status})`);
    return value;
  };
  let job = await request("/captures", "POST", capture);
  if (["failed", "partial", "interrupted", "cancelled"].includes(job.status))
    job = await request(`/captures/${job.id}/retry`, "POST");
  const deadline = Date.now() + 15 * 60 * 1000;
  let last = "";
  while (["pending", "running"].includes(job.status)) {
    if (last !== job.status) {
      report(job.status);
      last = job.status;
    }
    if (Date.now() >= deadline)
      throw Error(
        `Import ${job.id} is still queued or running; rerun to reconnect`,
      );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    job = await request(`/captures/${job.id}`);
  }
  if (job.status !== "completed")
    throw Error(
      `Import ${job.id} ${job.status}: ${job.error ?? job.result?.outcome?.reason ?? "check recent captures"}. Rerun to resume.`,
    );
  return job.result;
}
