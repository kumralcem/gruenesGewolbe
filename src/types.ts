export type Intent = "art" | "idea" | "capture" | "manage" | "ask" | "probe";
export interface Asset {
  bytes: string;
  width: number;
  height: number;
  visualHash: string;
  preview?: string;
}
export interface Draft {
  kind: "art" | "idea";
  captureKey?: string;
  instructions?: string;
  missingMedia?: string[];
  title: string;
  subvault: string;
  sourceUrl: string;
  summary: string;
  tags: string[];
  sourceText?: string;
  publishedAt?: string;
  creator?: string;
  year?: string;
  selectedImage?: string;
  assets?: Asset[];
}
export interface StoredAsset {
  file: string;
  hash: string;
  width: number;
  height: number;
  visualHash: string;
}
export interface Item extends Omit<Draft, "assets"> {
  id: string;
  capturedAt: string;
  assets: StoredAsset[];
  primary?: string;
}
export interface Outcome {
  status:
    | "saved"
    | "existing"
    | "upgraded"
    | "updated"
    | "partial"
    | "paused"
    | "queued"
    | "skipped"
    | "failed";
  path?: string;
  operationId?: string;
  conflicts?: string[];
  reason?: string;
  sourceUrl?: string;
}
export interface Hit {
  id: string;
  title: string;
  path: string;
  snippet: string;
  score: number;
  publishedAt?: string;
  capturedAt: string;
}
export interface Config {
  provider: "openai" | "openrouter" | "openai-codex";
  stateDir?: string;
  maxInputTokens?: number;
  limits?: Partial<import("./usage.ts").Limits>;
  model: string;
  api?: "openai-completions" | "openai-responses";
  apiKey?: string;
  maxRequests?: number;
  maxOutputTokens?: number;
  maxSeconds?: number;
  vision?: boolean;
}
export interface Job {
  intent: Intent;
  input: string;
  focus?: string;
  config: Omit<Config, "apiKey">;
  token: string;
  areas: string[];
  fixtures?: boolean;
  browserCapture?: boolean;
}
