import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { fileLock, readJson, writeJson } from "./state.ts";
export interface Limits {
  hourRequests: number;
  weekRequests: number;
  hourTokens: number;
  weekTokens: number;
}
export const defaultLimits: Limits = {
  hourRequests: 50,
  weekRequests: 300,
  hourTokens: 500000,
  weekTokens: 3000000,
};
interface State {
  attempts: { id: string; at: number; provider: string; tokens: number }[];
  providers: Record<
    string,
    {
      failures: number;
      paused?: string;
      allowance?: { usedPercent: number; resetAt?: number; observedAt: number };
    }
  >;
  override?: { requests: number; tokens: number; expires: number };
}
export class UsagePaused extends Error {
  constructor(
    message: string,
    readonly retryAt?: number,
  ) {
    super(message);
    this.name = "UsagePaused";
  }
}
export class UsageGuard {
  readonly limits: Limits;
  constructor(
    readonly dir: string,
    limits: Partial<Limits> = {},
    private now = Date.now,
  ) {
    this.limits = { ...defaultLimits, ...limits };
    if (
      Object.values(this.limits).some((n) => !Number.isSafeInteger(n) || n < 1)
    )
      throw Error("Usage limits must be positive integers");
  }
  private async state<T>(fn: (s: State) => T | Promise<T>) {
    return fileLock(join(this.dir, "usage.lock"), async () => {
      const s = await readJson<State>(join(this.dir, "usage.json"), {
        attempts: [],
        providers: {},
      });
      s.attempts = s.attempts.filter((a) => a.at > this.now() - 7 * 86400000);
      const value = await fn(s);
      await writeJson(join(this.dir, "usage.json"), s);
      return value;
    });
  }
  private windows(s: State) {
    return [
      {
        name: "hour",
        milliseconds: 3600000,
        requests: this.limits.hourRequests,
        tokens: this.limits.hourTokens,
      },
      {
        name: "week",
        milliseconds: 7 * 86400000,
        requests: this.limits.weekRequests,
        tokens: this.limits.weekTokens,
      },
    ].map((w) => {
      const entries = s.attempts.filter(
        (a) => a.at > this.now() - w.milliseconds,
      );
      return {
        ...w,
        usedRequests: entries.length,
        usedTokens: entries.reduce((n, a) => n + a.tokens, 0),
        nextReset: entries.length
          ? Math.min(...entries.map((a) => a.at)) + w.milliseconds
          : undefined,
      };
    });
  }
  async reserve(provider: string, tokens: number) {
    return this.state((s) => {
      if (!Number.isSafeInteger(tokens) || tokens < 1)
        throw Error("Invalid token reservation");
      if (s.providers[provider]?.paused)
        throw new UsagePaused(s.providers[provider].paused!);
      const blocked = this.windows(s).filter(
        (w) => w.usedRequests >= w.requests || w.usedTokens + tokens > w.tokens,
      );
      if (blocked.length) {
        if (
          s.override &&
          s.override.expires > this.now() &&
          s.override.requests > 0 &&
          s.override.tokens >= tokens
        ) {
          s.override.requests--;
          s.override.tokens -= tokens;
        } else
          throw new UsagePaused(
            "GG application usage budget reached; jobs paused",
            Math.max(
              ...blocked.map((w) => w.nextReset ?? this.now() + w.milliseconds),
            ),
          );
      }
      const id = randomUUID();
      s.attempts.push({ id, at: this.now(), provider, tokens });
      return id;
    });
  }
  async settle(id: string, tokens: number) {
    await this.state((s) => {
      const a = s.attempts.find((a) => a.id === id);
      if (a && Number.isSafeInteger(tokens) && tokens > 0) a.tokens = tokens;
    });
  }
  async success(provider: string) {
    await this.state((s) => {
      const p = (s.providers[provider] ??= { failures: 0 });
      p.failures = 0;
    });
  }
  async failure(provider: string, reason: string, immediate = false) {
    await this.state((s) => {
      const p = (s.providers[provider] ??= { failures: 0 });
      p.failures++;
      if (immediate || reason === "authentication" || p.failures >= 3)
        p.paused = reason;
    });
  }
  async allowance(provider: string, usedPercent: number, resetAt?: number) {
    if (Number.isFinite(usedPercent) && usedPercent >= 0 && usedPercent <= 100)
      await this.state((s) => {
        const p = (s.providers[provider] ??= { failures: 0 });
        p.allowance = { usedPercent, resetAt, observedAt: this.now() };
      });
  }
  async status() {
    return this.state((s) => {
      const windows = this.windows(s);
      return {
        windows,
        warning: windows.some(
          (w) =>
            w.usedRequests / w.requests >= 0.8 ||
            w.usedTokens / w.tokens >= 0.8,
        ),
        providers: s.providers,
        override:
          s.override && s.override.expires > this.now()
            ? s.override
            : undefined,
        tokenAccounting:
          "Reserved estimates until provider usage is reported; failed attempts retain reservations.",
      };
    });
  }
  async resume(provider: string) {
    await this.state((s) => {
      const p = (s.providers[provider] ??= { failures: 0 });
      delete p.paused;
      p.failures = 0;
    });
  }
  async override(requests: number, tokens: number, minutes = 60) {
    if (
      !Number.isSafeInteger(requests) ||
      requests < 1 ||
      requests > 50 ||
      !Number.isSafeInteger(tokens) ||
      tokens < 1 ||
      tokens > 1000000 ||
      !Number.isSafeInteger(minutes) ||
      minutes < 1 ||
      minutes > 60
    )
      throw Error(
        "Override allows 1–50 requests, up to 1M tokens, for at most 60 minutes",
      );
    await this.state((s) => {
      s.override = { requests, tokens, expires: this.now() + minutes * 60000 };
    });
  }
}
