import http from "node:http";
import { connect, type Socket } from "node:net";
import { mkdtemp, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { publicTarget, fetchPublic } from "./network.ts";
import { Vault } from "./vault.ts";
import type { Config, Draft, Intent, Outcome } from "./types.ts";
import { validateConfig } from "./config.ts";

type MockModel = (body: any) => Promise<any>;
export interface GatewayOptions {
  vault: Vault;
  config: Config;
  intent: Intent;
  input: string;
  fixtureFetch?: (
    url: string,
  ) => Promise<{ bytes: Buffer; type: string; url: string }>;
  fixturePage?: (url: string) => unknown;
  mockModel?: MockModel;
}
async function body(req: http.IncomingMessage, max: number) {
  let size = 0;
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    size += chunk.length;
    if (size > max) throw Error("Request too large");
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString());
}
export async function createGateway(options: GatewayOptions) {
  const { vault, config, intent } = options;
  validateConfig(config, intent);
  const dir = await mkdtemp(join(tmpdir(), "gg-gateway-"));
  const socket = join(dir, "broker.sock");
  const token = randomBytes(32).toString("hex");
  const abort = new AbortController();
  const timer = setTimeout(
    () => abort.abort(Error("Job deadline exceeded")),
    (config.maxSeconds ?? 180) * 1000,
  );
  const connections = new Set<Socket>();
  let requests = 0,
    networkBytes = 0,
    finishing = false;
  const outcomes: Outcome[] = [];
  const seen = new Set<string>();
  let answers: unknown[] = [];
  const events: Record<string, unknown>[] = [];
  const send = (res: http.ServerResponse, code: number, data: unknown) => {
    res.writeHead(code, { "content-type": "application/json" });
    res.end(JSON.stringify(data));
  };
  const server = http.createServer(async (req, res) => {
    try {
      events.push({
        type: "gateway_request",
        path: req.url,
        method: req.method,
      });
      if (abort.signal.aborted) throw Error("Job deadline exceeded");
      if (req.headers.authorization !== `Bearer ${token}`) {
        send(res, 401, { error: "Invalid job credential" });
        return;
      }
      if (req.method !== "POST") {
        send(res, 405, { error: "POST required" });
        return;
      }
      const route = req.url ?? "";
      const input = await body(
        req,
        route === "/save" ? 180_000_000 : 4_000_000,
      );
      if (["/save", "/queue", "/finish"].includes(route)) {
        if (input.sourceUrl !== options.input)
          throw Error("Outcome must belong to the submitted input");
        if (finishing || outcomes.length)
          throw Error("This input already has an outcome");
      }
      if (route === "/model/chat/completions" || route === "/model/responses") {
        // Admit only the request surface used by Pi's two configured APIs.
        // In particular, OpenRouter plugins/fallback models/provider routing
        // are separate fields, so restricting `tools` alone is insufficient.
        const fields = new Set([
          "model",
          "messages",
          "input",
          "instructions",
          "stream",
          "stream_options",
          "max_output_tokens",
          "max_tokens",
          "max_completion_tokens",
          "temperature",
          "top_p",
          "tools",
          "tool_choice",
          "parallel_tool_calls",
          "store",
          "reasoning",
          "reasoning_effort",
          "text",
          "include",
          "prompt_cache_key",
          "prompt_cache_retention",
          "prompt_cache_options",
          "metadata",
          "safety_identifier",
          "service_tier",
          "user",
          "truncation",
          "n",
        ]);
        if (Object.keys(input).some((key) => !fields.has(key)))
          throw Error(
            "Unsupported model request field; provider extensions and routing overrides are disabled",
          );
        if (input.n !== undefined && input.n !== 1)
          throw Error("Only one model completion per request is allowed");
        if (
          input.background ||
          input.previous_response_id ||
          input.conversation
        )
          throw Error("Only foreground stateless model requests are allowed");
        if (
          input.service_tier &&
          input.service_tier !== "default" &&
          input.service_tier !== "auto"
        )
          throw Error("Extra provider service tiers are not enabled");
        if (
          input.tools !== undefined &&
          (!Array.isArray(input.tools) ||
            input.tools.length > 64 ||
            input.tools.some((tool: any) => tool?.type !== "function"))
        )
          throw Error(
            "Only local function tools are allowed; provider-hosted tools are disabled",
          );
        input.store = false;
        if (++requests > (config.maxRequests ?? 10))
          throw Error("Model request budget exceeded");
        if (input.model !== config.model)
          throw Error("Model is fixed by user configuration for this job");
        const max = config.maxOutputTokens ?? 4096;
        if (route.endsWith("/responses"))
          input.max_output_tokens = Math.min(
            input.max_output_tokens ?? max,
            max,
          );
        else if ("max_completion_tokens" in input)
          input.max_completion_tokens = Math.min(
            input.max_completion_tokens ?? max,
            max,
          );
        else input.max_tokens = Math.min(input.max_tokens ?? max, max);
        if (
          ["max_output_tokens", "max_tokens", "max_completion_tokens"].some(
            (k) =>
              k in input &&
              (!Number.isSafeInteger(input[k]) ||
                input[k] < 1 ||
                input[k] > max),
          )
        )
          throw Error("Invalid output token limit");
        events.push({
          type: "model_request",
          request: requests,
          provider: config.provider,
          model: config.model,
        });
        if (options.mockModel) {
          const answer = await options.mockModel(input);
          if (input.stream) {
            res.writeHead(200, { "content-type": "text/event-stream" });
            const delta = answer.tool_calls
              ? {
                  role: "assistant",
                  content: null,
                  tool_calls: answer.tool_calls.map((t: any, i: number) => ({
                    index: i,
                    ...t,
                  })),
                }
              : { role: "assistant", content: answer.content };
            const base = {
              id: `fixture-${requests}`,
              object: "chat.completion.chunk",
              created: 1,
              model: config.model,
            };
            res.write(
              `data: ${JSON.stringify({ ...base, choices: [{ index: 0, delta, finish_reason: null }] })}\n\n`,
            );
            res.end(
              `data: ${JSON.stringify({ ...base, choices: [{ index: 0, delta: {}, finish_reason: answer.tool_calls ? "tool_calls" : "stop" }], usage: { prompt_tokens: 100, completion_tokens: 100, total_tokens: 200 } })}\n\ndata: [DONE]\n\n`,
            );
          } else
            send(res, 200, {
              id: "fixture",
              object: "chat.completion",
              model: config.model,
              choices: [
                {
                  index: 0,
                  message: answer,
                  finish_reason: answer.tool_calls ? "tool_calls" : "stop",
                },
              ],
              usage: {
                prompt_tokens: 100,
                completion_tokens: 100,
                total_tokens: 200,
              },
            });
          return;
        }
        if (!config.apiKey) throw Error("Provider API key is not configured");
        const upstream =
          (config.provider === "openai"
            ? "https://api.openai.com/v1"
            : "https://openrouter.ai/api/v1") + route.slice("/model".length);
        const response = await fetch(upstream, {
          method: "POST",
          signal: abort.signal,
          headers: {
            "content-type": "application/json",
            authorization: `Bearer ${config.apiKey}`,
          },
          body: JSON.stringify(input),
        });
        if (!response.ok) {
          const detail = (await response.json().catch(() => ({}))) as any;
          const message =
            typeof detail.error?.message === "string"
              ? detail.error.message
                  .replaceAll(config.apiKey, "[redacted]")
                  .slice(0, 700)
              : "";
          send(res, response.status, {
            error: `Provider returned HTTP ${response.status}${message ? ": " + message : ""}`,
          });
          return;
        }
        res.writeHead(200, {
          "content-type":
            response.headers.get("content-type") ?? "application/json",
        });
        if (response.body)
          for await (const chunk of response.body) res.write(chunk);
        res.end();
        return;
      }
      if (route === "/search") {
        const hits = await vault.search(input.query);
        hits.forEach((h) => seen.add(h.id));
        send(res, 200, hits);
        return;
      }
      if (route === "/read") {
        if (!seen.has(input.id)) throw Error("Search before reading an item");
        send(res, 200, await vault.read(input.id, input.offset));
        return;
      }
      if (route === "/results" && intent === "ask") {
        if (!Array.isArray(input.items) || input.items.length > 8)
          throw Error("Invalid results");
        answers = await Promise.all(
          input.items.map(async (v: any) => {
            if (!seen.has(v.id) || typeof v.reason !== "string")
              throw Error("Unknown result ID");
            const { item, path } = await vault.read(v.id);
            return {
              id: item.id,
              title: item.title,
              path,
              publishedAt: item.publishedAt,
              capturedAt: item.capturedAt,
              reason: v.reason.slice(0, 1000),
            };
          }),
        );
        send(res, 200, answers);
        return;
      }
      if (intent === "ask" && !["/finish"].includes(route))
        throw Error(
          "Ask job has read-only archive access and no website access",
        );
      if (route === "/fetch") {
        const result = options.fixtureFetch
          ? await options.fixtureFetch(input.url)
          : await fetchPublic(input.url, abort.signal);
        networkBytes += result.bytes.length;
        if (networkBytes > 150_000_000)
          throw Error("Job download budget exceeded");
        send(res, 200, { ...result, bytes: result.bytes.toString("base64") });
        return;
      }
      if (route === "/fixture-page" && options.fixturePage) {
        send(res, 200, options.fixturePage(input.url));
        return;
      }
      if (route === "/save") {
        if (outcomes.length) throw Error("This input already has an outcome");
        if (intent !== "art" && intent !== "idea")
          throw Error("No capture capability");
        if (input.kind !== intent) throw Error("Capture intent mismatch");
        finishing = true;
        try {
          const result = await vault.save(input as Draft);
          outcomes.push(result);
          send(res, 200, result);
        } finally {
          finishing = false;
        }
        return;
      }
      if (route === "/queue") {
        if (outcomes.length) throw Error("This input already has an outcome");
        if (intent !== "art" && intent !== "idea")
          throw Error("No queue capability");
        finishing = true;
        try {
          const result = await vault.queue(
            input.sourceUrl,
            input.reason,
            input.candidates,
          );
          outcomes.push(result);
          send(res, 200, result);
        } finally {
          finishing = false;
        }
        return;
      }
      if (route === "/finish") {
        if (outcomes.length) throw Error("This input already has an outcome");
        if (
          !["skipped", "failed"].includes(input.status) ||
          typeof input.reason !== "string"
        )
          throw Error("Invalid completion status");
        outcomes.push({
          status: input.status,
          reason: input.reason.slice(0, 1000),
          sourceUrl: options.input,
        });
        send(res, 200, { ok: true });
        return;
      }
      send(res, 404, { error: "Unknown operation" });
    } catch (error) {
      if (!res.headersSent)
        send(res, 400, {
          error: error instanceof Error ? error.message : "Request rejected",
        });
      else res.destroy();
    }
  });
  server.on("connection", (s) => {
    connections.add(s);
    s.on("close", () => connections.delete(s));
  });
  server.on("connect", async (req, client, head) => {
    try {
      if (intent === "ask" || abort.signal.aborted || connections.size > 24)
        throw Error("Network capability denied");
      const { url, address } = await publicTarget(`https://${req.url}`);
      if (abort.signal.aborted || client.destroyed)
        throw Error("Job ended during DNS lookup");
      if (url.port && url.port !== "443")
        throw Error("HTTPS proxy port denied");
      const remote = connect({ host: address.address, port: 443 });
      connections.add(remote);
      remote.on("close", () => connections.delete(remote));
      remote.setTimeout(30000, () => remote.destroy());
      remote.on("error", () => client.destroy());
      client.on("error", () => remote.destroy());
      remote.on("connect", () => {
        client.write("HTTP/1.1 200 Connection Established\r\n\r\n");
        if (head.length) remote.write(head);
        client.pipe(remote);
        remote.pipe(client);
      });
      remote.on("data", (b: Buffer) => {
        networkBytes += b.length;
        if (networkBytes > 150_000_000)
          abort.abort(Error("Network budget exceeded"));
      });
    } catch {
      client.end("HTTP/1.1 403 Forbidden\r\n\r\n");
    }
  });
  abort.signal.addEventListener("abort", () => {
    for (const s of connections) s.destroy();
  });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(socket, resolve);
  });
  await chmod(socket, 0o600);
  return {
    socket,
    token,
    events,
    outcomes,
    signal: abort.signal,
    get answers() {
      return answers;
    },
    get requests() {
      return requests;
    },
    async close() {
      clearTimeout(timer);
      abort.abort();
      await new Promise<void>((r) => server.close(() => r()));
      await rm(dir, { recursive: true, force: true });
    },
  };
}
