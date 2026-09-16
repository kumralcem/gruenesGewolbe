import type { Config, Intent } from "./types.ts";

export function validateConfig(config: Config, intent: Intent) {
  if (!["openai", "openrouter", "openai-codex"].includes(config.provider))
    throw Error("Unsupported provider");
  if (
    intent !== "probe" &&
    (typeof config.model !== "string" ||
      !config.model.trim() ||
      config.model.length > 300)
  )
    throw Error("Provide a model slug");
  if (
    config.api &&
    !["openai-completions", "openai-responses"].includes(config.api)
  )
    throw Error("Unsupported API format");
  if (config.vision !== undefined && typeof config.vision !== "boolean")
    throw Error("vision must be true or false");
  if (intent === "art" && config.vision === false)
    throw Error("Art capture requires a model configured for image input");
  for (const [label, value, ceiling] of [
    ["maxInputTokens", config.maxInputTokens ?? 64000, 128000],
    ["maxSeconds", config.maxSeconds ?? 180, 600],
    ["maxRequests", config.maxRequests ?? 10, 30],
    ["maxOutputTokens", config.maxOutputTokens ?? 4096, 16384],
  ] as const) {
    if (!Number.isSafeInteger(value) || value < 1 || value > ceiling)
      throw Error(`${label} must be an integer from 1 to ${ceiling}`);
  }
}
