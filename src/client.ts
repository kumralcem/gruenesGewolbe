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
