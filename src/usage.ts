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
  attempts: {
    id: string;
    at: number;
    provider: string;
    tokens: number;
    hour?: number;
    week?: number;
    job?: string;
    input?: string;
    grant?: string;
  }[];
  epochs?: { hour: number; week: number };
  limits?: Limits;
  actions?: { at: number; action: string; detail: unknown }[];
  grants?: Record<
    string,
    { requests: number; tokens: number; expires: number; sources: string[] }
  >;
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
        requests: (s.limits ?? this.limits).hourRequests,
        tokens: (s.limits ?? this.limits).hourTokens,
      },
      {
        name: "week",
        milliseconds: 7 * 86400000,
        requests: (s.limits ?? this.limits).weekRequests,
        tokens: (s.limits ?? this.limits).weekTokens,
      },
    ].map((w) => {
      const entries = s.attempts.filter(
        (a) =>
          a.at > this.now() - w.milliseconds &&
          (a[w.name as "hour" | "week"] ?? 0) ===
            (s.epochs?.[w.name as "hour" | "week"] ?? 0),
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
  async reserve(
    provider: string,
    tokens: number,
    context?: { job?: string; input?: string; grant?: string },
  ) {
    return this.state((s) => {
      if (!Number.isSafeInteger(tokens) || tokens < 1)
        throw Error("Invalid token reservation");
      if (s.providers[provider]?.paused)
        throw new UsagePaused(s.providers[provider].paused!);
      const blocked = this.windows(s).filter(
        (w) => w.usedRequests >= w.requests || w.usedTokens + tokens > w.tokens,
      );
      if (context?.grant) {
        const grant = s.grants?.[context.grant];
        if (
          !grant ||
          grant.expires <= this.now() ||
          !grant.sources.includes(context.input ?? "")
        )
          throw new UsagePaused(
            "Collection allowance is expired, missing, or does not include this image",
          );
        if (grant.requests < 1 || grant.tokens < tokens)
          throw new UsagePaused("Collection allowance exhausted");
        grant.requests--;
        grant.tokens -= tokens;
      } else if (blocked.length) {
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
            `GG application usage budget reached: ${blocked.map((w) => w.name + (w.usedRequests >= w.requests ? " request" : " token") + " limit").join(" and ")} reached. Run gg usage for recovery options.`,
            Math.max(
              ...blocked.map((w) => w.nextReset ?? this.now() + w.milliseconds),
            ),
          );
      }
      const id = randomUUID();
      s.attempts.push({
        id,
        at: this.now(),
        provider,
        tokens,
        ...context,
        hour: s.epochs?.hour ?? 0,
        week: s.epochs?.week ?? 0,
      });
      return id;
    });
  }
  async settle(id: string, tokens: number) {
    await this.state((s) => {
      const a = s.attempts.find((a) => a.id === id);
      if (a && Number.isSafeInteger(tokens) && tokens > 0) {
        if (a.grant && s.grants?.[a.grant])
          s.grants[a.grant].tokens += a.tokens - tokens;
        a.tokens = tokens;
      }
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
        grants: s.grants ?? {},
        limits: s.limits ?? this.limits,
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
  async reset(window: string) {
    if (!["hour", "week", "all"].includes(window))
      throw Error("Choose --window hour, week or all");
    await this.state((s) => {
      s.epochs ??= { hour: 0, week: 0 };
      if (window !== "week") s.epochs.hour++;
      if (window !== "hour") s.epochs.week++;
      (s.actions ??= []).push({
        at: this.now(),
        action: "reset",
        detail: window,
      });
    });
  }
  async setLimits(limits: Partial<Limits>) {
    if (
      Object.keys(limits).some((k) => !(k in defaultLimits)) ||
      Object.values(limits).some((v) => !Number.isSafeInteger(v) || v! < 1)
    )
      throw Error("Limits must be positive integers with supported names");
    await this.state((s) => {
      s.limits = { ...(s.limits ?? this.limits), ...limits };
      (s.actions ??= []).push({
        at: this.now(),
        action: "limits",
        detail: s.limits,
      });
    });
  }
  async grant(
    requests: number,
    tokens: number,
    minutes: number,
    sources: string[],
  ) {
    if (
      !Number.isSafeInteger(requests) ||
      requests < 1 ||
      requests > 10000 ||
      !Number.isSafeInteger(tokens) ||
      tokens < 1 ||
      tokens > 2_000_000_000 ||
      !Number.isSafeInteger(minutes) ||
      minutes < 1 ||
      minutes > 1440 ||
      !sources.length ||
      sources.length > 2000 ||
      sources.some((s) => !/^gg-local:sha256:[a-f0-9]{64}$/.test(s))
    )
      throw Error(
        "Collection grant requires 1–10000 requests, 1–2B tokens, 1–1440 minutes and up to 2000 image hashes",
      );
    const id = randomUUID();
    await this.state((s) => {
      (s.grants ??= {})[id] = {
        requests,
        tokens,
        expires: this.now() + minutes * 60000,
        sources: [...new Set(sources)],
      };
      (s.actions ??= []).push({
        at: this.now(),
        action: "grant",
        detail: { id, requests, tokens, minutes, images: sources.length },
      });
    });
    return id;
  }
  async revokeGrant(id: string) {
    await this.state((s) => {
      if (!s.grants?.[id]) throw Error("Unknown allowance");
      delete s.grants[id];
      (s.actions ??= []).push({ at: this.now(), action: "revoke", detail: id });
    });
  }
  async history() {
    return this.state((s) => {
      const jobs = new Map<
        string,
        {
          job: string;
          input?: string;
          provider: string;
          requests: number;
          tokens: number;
          lastAt: number;
        }
      >();
      for (const a of s.attempts) {
        const key = a.job ?? "legacy-unassigned";
        const row = jobs.get(key) ?? {
          job: key,
          input: a.input,
          provider: a.provider,
          requests: 0,
          tokens: 0,
          lastAt: a.at,
        };
        row.requests++;
        row.tokens += a.tokens;
        row.lastAt = Math.max(row.lastAt, a.at);
        jobs.set(key, row);
      }
      return {
        jobs: [...jobs.values()].sort((a, b) => b.lastAt - a.lastAt),
        actions: s.actions ?? [],
        retention: "Requests: 7 days; administrative actions retained",
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
      (s.actions ??= []).push({
        at: this.now(),
        action: "temporary-grant",
        detail: { requests, tokens, minutes },
      });
    });
  }
}
