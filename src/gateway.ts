import { ModelService } from "./model-service.ts";
import { defaultStateDir, readJson, writeJson } from "./state.ts";
import { UsagePaused } from "./usage.ts";
import { fixtureCompletion } from "./model-bridge.ts";
import { createHash, randomUUID } from "node:crypto";
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

import type { BrowserCapture } from "./browser-capture.ts";

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
  browserCapture?: BrowserCapture;
  batchId?: string;
  instructions?: string;
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
  await vault.destinations();
  const captureRules =
    intent === "capture" ? await vault.captureRules() : undefined;
  let batchId = options.batchId ?? randomUUID();
  const revision = createHash("sha256")
    .update(
      JSON.stringify(
        options.browserCapture ?? { input: options.input, batchId },
      ),
    )
    .digest("hex");
  const planPath = join(vault.root, ".gg-plans", revision + ".json");
  const storedPlan =
    intent === "capture"
      ? await readJson<
          | {
              sourceUrl: string;
              keys: string[];
              batchId?: string;
              completed?: Record<string, Outcome>;
            }
          | undefined
        >(planPath, undefined)
      : undefined;
  if (
    storedPlan &&
    (storedPlan.sourceUrl !== options.input ||
      !Array.isArray(storedPlan.keys) ||
      !storedPlan.keys.length ||
      storedPlan.keys.length > 20 ||
      !storedPlan.keys.every(
        (k) => typeof k === "string" && /^[a-zA-Z0-9_-]{1,100}$/.test(k),
      ))
  )
    throw Error("Invalid persisted capture plan");
  let planned = storedPlan?.keys ?? ["source"];
  let planFrozen = Boolean(storedPlan);
  if (storedPlan?.batchId) batchId = storedPlan.batchId;
  const completed: Record<string, Outcome> = Object.assign(
    Object.create(null),
    storedPlan?.completed ?? {},
  );
  const savedKeys = new Set<string>();
  const previousOutcomes: Outcome[] = [];
  const initiallySaved = new Map<string, Outcome>();
  for (const [key, value] of Object.entries(completed)) {
    if (!planned.includes(key) || !value || typeof value.path !== "string")
      throw Error("Invalid completed capture key");
    const outcome: Outcome = {
      ...value,
      status: value.status === "partial" ? "partial" : "existing",
    };
    savedKeys.add(key);
    initiallySaved.set(key, outcome);
    previousOutcomes.push(outcome);
  }
  if (intent === "capture" && storedPlan) {
    for (const record of await vault.sourceRecords(options.input)) {
      const baseline = await readJson<{ revision?: string } | undefined>(
        join(record.path, ".gg-baseline.json"),
        undefined,
      );
      const key = record.item.captureKey ?? "source";
      if (
        !savedKeys.has(key) &&
        baseline?.revision === revision &&
        planned.includes(key)
      ) {
        const outcome: Outcome = { status: "existing", path: record.path };
        savedKeys.add(key);
        initiallySaved.set(key, outcome);
        previousOutcomes.push(outcome);
        completed[key] = outcome;
      }
    }
  }
  const persistPlan = async (keys: string[]) => {
    if (planFrozen && JSON.stringify(keys) !== JSON.stringify(planned))
      throw Error(
        "Retry must retain the original capture plan; save only pending keys",
      );
    await writeJson(planPath, {
      sourceUrl: options.input,
      keys,
      batchId,
      completed,
    });
    planned = keys;
    planFrozen = true;
  };
  const dir = await mkdtemp(join(tmpdir(), "gg-gateway-"));
  const socket = join(dir, "broker.sock");
  const token = randomBytes(32).toString("hex");
  const abort = new AbortController();
  const timer = setTimeout(
    () => abort.abort(Error("Job deadline exceeded")),
    (config.maxSeconds ?? 180) * 1000,
  );
  const connections = new Set<Socket>();
  const service = new ModelService(
    config.stateDir ?? defaultStateDir(),
    config,
  );
  const management: unknown[] = [];
  let paused: string | undefined;
  let retryAt: number | undefined;
  let requests = 0,
    networkBytes = 0,
    finishing = false;
  const outcomes: Outcome[] = previousOutcomes;
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
        if (finishing || (intent !== "capture" && outcomes.length))
          throw Error("This input already has an outcome");
      }
      if (route === "/model/pi") {
        const attempt = () => {
          if (requests >= (config.maxRequests ?? 10))
            throw Error("Model request budget exceeded");
          requests++;
          events.push({
            type: "model_request",
            request: requests,
            provider: config.provider,
            model: config.model,
          });
        };
        if (options.mockModel) {
          attempt();
          send(
            res,
            200,
            await fixtureCompletion(
              input.context,
              config.model,
              options.mockModel,
            ),
          );
        } else {
          try {
            send(
              res,
              200,
              await service.complete(input.context, abort.signal, attempt),
            );
          } catch (e) {
            if (e instanceof UsagePaused) {
              paused = e.message;
              retryAt = e.retryAt;
            }
            throw e;
          }
        }
        return;
      }
      if (route === "/capture-context" && intent === "capture") {
        const records = await vault.sourceRecords(options.input);
        send(res, 200, {
          captureRules: captureRules?.text,
          records: records.map((v) => ({
            id: v.item.id,
            key: v.item.captureKey ?? "source",
            title: v.item.title,
            summary: v.item.summary.slice(0, 1000),
          })),
          plannedKeys: planned,
          completedKeys: [...savedKeys],
          pendingKeys: planned.filter((k) => !savedKeys.has(k)),
          instructions:
            options.instructions ??
            options.browserCapture?.instructions ??
            records[0]?.item.instructions,
        });
        return;
      }
      if (route === "/plan" && intent === "capture") {
        if (
          !Array.isArray(input.keys) ||
          !input.keys.length ||
          input.keys.length > 20 ||
          new Set(input.keys).size !== input.keys.length ||
          !input.keys.every(
            (k: unknown) =>
              typeof k === "string" && /^[a-zA-Z0-9_-]{1,100}$/.test(k),
          )
        )
          throw Error("Invalid capture plan");
        await persistPlan(input.keys);
        send(res, 200, {
          keys: planned,
          pendingKeys: planned.filter((k) => !savedKeys.has(k)),
        });
        return;
      }
      if (route === "/catalog" && intent === "manage") {
        const offset = input.offset ?? 0;
        if (!Number.isSafeInteger(offset) || offset < 0)
          throw Error("Invalid catalog offset");
        const all = await vault.items();
        const records = all.slice(offset, offset + 50);
        records.forEach((v) => seen.add(v.item.id));
        send(res, 200, {
          subvaults: vault.areas,
          records: records.map((v) => ({
            id: v.item.id,
            title: v.item.title,
            subvault: v.item.subvault,
            sourceUrl: v.item.sourceUrl,
          })),
          history: (await vault.history()).slice(0, 20),
          nextOffset: offset + 50 < all.length ? offset + 50 : null,
        });
        return;
      }
      if (route === "/create-destination" && intent === "capture") {
        const instructions =
          options.instructions ??
          options.browserCapture?.instructions ??
          (await vault.sourceRecords(options.input))[0]?.item.instructions;
        if (!instructions?.trim())
          throw Error(
            "Destination creation requires user capture instructions",
          );
        if (typeof input.subvault !== "string")
          throw Error("Provide a destination path");
        if ((await vault.destinations()).includes(input.subvault)) {
          send(res, 200, { existing: true, subvault: input.subvault });
          return;
        }
        const result = await vault.manage(
          { action: "create-subvault", subvault: input.subvault },
          batchId,
        );
        management.push(result);
        send(res, 200, { ...result, subvault: input.subvault });
        return;
      }
      if (route === "/manage" && intent === "manage") {
        const result = await vault.manage(input, batchId);
        management.push(result);
        send(res, 200, result);
        return;
      }
      if (route === "/undo" && intent === "manage") {
        const result = await vault.undo(input.id);
        management.push({ undone: result });
        send(res, 200, result);
        return;
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
        throw Error("Live model calls must use the controller Pi gateway");
      }
      if (
        route === "/browser-capture" &&
        options.browserCapture &&
        intent !== "ask"
      ) {
        send(res, 200, options.browserCapture);
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
      if (
        (intent === "ask" || intent === "manage") &&
        !["/finish"].includes(route)
      )
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
        if (intent !== "capture" && outcomes.length)
          throw Error("This input already has an outcome");
        if (!["art", "idea", "capture"].includes(intent))
          throw Error("No capture capability");
        if (intent !== "capture" && input.kind !== intent)
          throw Error("Capture intent mismatch");
        if (intent === "capture") {
          const key = input.captureKey ?? "source";
          if (!planned.includes(key))
            throw Error(
              "Declare record keys with plan_capture before splitting",
            );
          if (initiallySaved.has(key)) {
            send(res, 200, initiallySaved.get(key));
            return;
          }
          if (savedKeys.has(key))
            throw Error("Record already saved in this job");
          await persistPlan(planned);
          if (
            input.includeImages !== undefined &&
            typeof input.includeImages !== "boolean"
          )
            throw Error("Invalid image policy");
          if (
            input.includeImages === false &&
            (input.kind !== "idea" ||
              (input.assets ?? []).length ||
              (input.missingMedia ?? []).length)
          )
            throw Error(
              "Text-only capture must be an idea with no image assets or media errors",
            );
          const snapshot = options.browserCapture;
          if (snapshot) {
            const preservedText = [
              snapshot.text,
              snapshot.transcript,
              input.includeContext ? snapshot.contextText : undefined,
            ]
              .filter(Boolean)
              .join("\n\n");
            input.sourceText = preservedText.slice(0, 400000);
            if (
              !Array.isArray(input.missingMedia ?? []) ||
              (input.missingMedia ?? []).length > 24 ||
              (input.missingMedia ?? []).some(
                (m: unknown) => typeof m !== "string" || m.length > 4000,
              )
            )
              throw Error("Invalid missing-media report");
            input.missingMedia = [
              ...(input.missingMedia ?? []),
              ...(preservedText.length > 400000
                ? ["Preserved source was truncated at 400,000 characters."]
                : []),
              ...(snapshot.warnings ?? []).filter(
                (warning) =>
                  input.includeImages !== false ||
                  warning !==
                    "Only the first 24 relevant image candidates were collected.",
              ),
              ...(input.includeImages === false ? [] : (snapshot.images ?? []))
                .filter((i) => !i.bytes)
                .map((i) => `${i.url}: ${i.error ?? "unavailable"}`),
            ];
            const provided = new Set(
              (snapshot.images ?? [])
                .filter((i) => i.bytes)
                .map((i) =>
                  createHash("sha256")
                    .update(Buffer.from(i.bytes!, "base64"))
                    .digest("hex"),
                ),
            );
            // Captured media must come from this snapshot; optional public research
            // is context only and cannot replace the user's captured material.
            if (
              (input.assets ?? []).some(
                (a: any) =>
                  !provided.has(
                    createHash("sha256")
                      .update(Buffer.from(a.bytes, "base64"))
                      .digest("hex"),
                  ),
              )
            )
              throw Error("Preserve supplied browser image bytes");
          }
          input.instructions =
            options.instructions ??
            snapshot?.instructions ??
            (await vault.sourceRecords(options.input))[0]?.item.instructions;
          finishing = true;
          try {
            const saved = await vault.capture(input, revision, batchId);
            const result: Outcome = input.missingMedia?.length
              ? {
                  ...saved,
                  status: "partial",
                  reason:
                    "Saved available content; missing media is listed in the record. Recapture to supply missing bytes.",
                }
              : saved;
            outcomes.push(result);
            savedKeys.add(key);
            completed[key] = result;
            await persistPlan(planned);
            send(res, 200, result);
          } finally {
            finishing = false;
          }
          return;
        }
        const selected = options.browserCapture?.image;
        if (intent === "art" && selected) {
          const original = Buffer.from(selected.bytes, "base64");
          if (
            input.selectedImage !== selected.url ||
            !Array.isArray(input.assets) ||
            !input.assets.some(
              (asset: any) =>
                typeof asset?.bytes === "string" &&
                Buffer.from(asset.bytes, "base64").equals(original),
            )
          )
            throw Error(
              "Preserve the exact browser-selected image and its reference",
            );
        }
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
    batchId,
    management,
    get paused() {
      return paused;
    },
    get retryAt() {
      return retryAt;
    },
    get complete() {
      return planned.every((key) => savedKeys.has(key));
    },
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
