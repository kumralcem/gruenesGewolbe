import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import type {
  Context,
  AssistantMessage,
  AuthInteraction,
} from "@earendil-works/pi-ai";
import { join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { privateDirectory } from "./state.ts";
import { UsageGuard, UsagePaused } from "./usage.ts";
import type { Config } from "./types.ts";

export async function providerRuntime(dir: string) {
  await privateDirectory(dir);
  return ModelRuntime.create({
    authPath: join(dir, "auth.json"),
    modelsPath: null,
    modelsStorePath: join(dir, "models.json"),
    allowModelNetwork: false,
  });
}
export async function loginProvider(
  dir: string,
  provider: string,
  interaction: AuthInteraction,
) {
  const runtime = await providerRuntime(dir);
  await runtime.login(
    provider,
    provider === "openai-codex" ? "oauth" : "api_key",
    interaction,
  );
}
export function estimateInput(context: Context) {
  let imageTokens = 0;
  const text = JSON.stringify(context, (key, value) => {
    if (value && typeof value === "object" && value.type === "image") {
      imageTokens += 8192;
      return { type: "image" };
    }
    return value;
  });
  // UTF-8 bytes conservatively bound ordinary text tokens. Image accounting is
  // an explicit estimate; reported provider usage replaces reservations.
  return Buffer.byteLength(text) + imageTokens;
}
export class ModelService {
  readonly usage: UsageGuard;
  private runtime?: Promise<ModelRuntime>;
  constructor(
    readonly dir: string,
    readonly config: Config,
    private completeProvider?: ModelRuntime["completeSimple"],
  ) {
    this.usage = new UsageGuard(dir, config.limits);
  }
  async complete(
    context: Context,
    signal: AbortSignal,
    onAttempt: () => void,
  ): Promise<AssistantMessage> {
    if (
      !context ||
      !Array.isArray(context.messages) ||
      context.messages.length > 300 ||
      (context.tools &&
        (!Array.isArray(context.tools) ||
          context.tools.length > 64 ||
          context.tools.some(
            (t) =>
              !t ||
              typeof t.name !== "string" ||
              typeof t.parameters !== "object",
          )))
    )
      throw Error("Invalid model context");
    const inputTokens = estimateInput(context);
    if (inputTokens > (this.config.maxInputTokens ?? 64000))
      throw Error("Model input exceeds configured bound");
    const runtime = await (this.runtime ??= providerRuntime(this.dir));
    if (this.config.apiKey)
      await runtime.setRuntimeApiKey(this.config.provider, this.config.apiKey);
    let model = runtime.getModel(this.config.provider, this.config.model);
    if (!model && this.config.provider !== "openai-codex") {
      runtime.registerProvider(this.config.provider, {
        baseUrl:
          this.config.provider === "openai"
            ? "https://api.openai.com/v1"
            : "https://openrouter.ai/api/v1",
        api:
          this.config.api ??
          (this.config.provider === "openai"
            ? "openai-responses"
            : "openai-completions"),
        models: [
          {
            id: this.config.model,
            name: this.config.model,
            reasoning: false,
            input: this.config.vision === false ? ["text"] : ["text", "image"],
            contextWindow: 128000,
            maxTokens: this.config.maxOutputTokens ?? 4096,
            cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
          },
        ],
      });
      model = runtime.getModel(this.config.provider, this.config.model);
    }
    if (!model)
      throw Error(
        "Model is not in Pi's provider catalog; run gg models to choose an available model",
      );
    if (this.config.provider !== "openai-codex" && this.config.api)
      model = { ...model, api: this.config.api };
    for (let attempt = 0; attempt < 3; attempt++) {
      signal.throwIfAborted();
      onAttempt();
      const reservation = await this.usage.reserve(
        this.config.provider,
        inputTokens +
          (this.config.provider === "openai-codex"
            ? model.maxTokens
            : (this.config.maxOutputTokens ?? 4096)),
        { ...this.config.usageJob, grant: this.config.usageGrant },
      );
      let status = 0;
      try {
        const response = await (
          this.completeProvider ?? runtime.completeSimple.bind(runtime)
        )(model, context, {
          signal,
          transport: "sse",
          maxRetries: 0,
          maxTokens: this.config.maxOutputTokens ?? 4096,
          reasoning: "low",
          timeoutMs: 60000,
          onResponse: async (r) => {
            status = r.status;
            const windows = ["primary", "secondary"]
              .map((w) => ({
                used: r.headers[`x-codex-${w}-used-percent`],
                reset: r.headers[`x-codex-${w}-reset-at`],
              }))
              .filter((w) => w.used !== undefined);
            if (windows.length) {
              const max = windows.reduce((a, b) =>
                Number(a.used) >= Number(b.used) ? a : b,
              );
              await this.usage.allowance(
                this.config.provider,
                Number(max.used),
                max.reset ? Number(max.reset) * 1000 : undefined,
              );
            }
          },
        });
        if (
          response.stopReason === "error" ||
          response.stopReason === "aborted"
        )
          throw Error(response.errorMessage ?? "Model request failed");
        const total =
          response.usage.input +
          response.usage.output +
          response.usage.cacheRead +
          response.usage.cacheWrite;
        await this.usage.settle(reservation, total);
        await this.usage.success(this.config.provider);
        return response;
      } catch (error) {
        if (signal.aborted) throw error;
        const message = error instanceof Error ? error.message : String(error);
        const immediate =
          status === 401 ||
          status === 403 ||
          /usage limit|insufficient.quota|quota exceeded|billing|not authenticated|not configured|oauth|unauthorized/i.test(
            message,
          );
        const transient =
          status === 429 ||
          status >= 500 ||
          /network|fetch failed|ECONN|timeout|timed out/i.test(message);
        await this.usage.failure(
          this.config.provider,
          immediate
            ? "Provider authentication or allowance requires attention"
            : `Repeated provider failures (${status || "network"})`,
          immediate,
        );
        if (immediate)
          throw new UsagePaused(
            "Provider authentication or allowance requires attention; sign in or resume manually",
          );
        if ((await this.usage.status()).providers[this.config.provider]?.paused)
          throw new UsagePaused(
            "Repeated provider failures; inspect the provider and resume manually",
          );
        if (!transient || attempt === 2)
          throw Error(
            this.config.apiKey
              ? message.replaceAll(this.config.apiKey, "[redacted]")
              : message,
          );
        await delay(1000 * 2 ** attempt, undefined, { signal });
      }
    }
    throw Error("Model attempts exhausted");
  }
}
