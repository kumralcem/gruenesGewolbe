import { isLocalSource } from "./local-source.ts";
import { validateBrowserCapture } from "./browser-capture.ts";
import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { Vault } from "./vault.ts";
import { runJob } from "./runner.ts";
import { fixtures } from "./fixtures.ts";
import { UsageGuard } from "./usage.ts";
import { validateConfig } from "./config.ts";
import type { Config, Intent } from "./types.ts";
import type { BrowserCapture } from "./browser-capture.ts";
import { writeJson } from "./state.ts";
export interface Command {
  command: string;
  text?: string;
  id?: string;
  instructions?: string;
  provider?: Config["provider"];
  model?: string;
  requests?: number;
  tokens?: number;
  minutes?: number;
}
export class Controller {
  readonly usage: UsageGuard;
  constructor(
    readonly vault: Vault,
    readonly config: Config,
    readonly stateDir: string,
    private fixture = false,
    private fixtureImage?: string,
  ) {
    this.usage = new UsageGuard(stateDir, config.limits);
  }
  async run(
    intent: Intent,
    input: string,
    signal?: AbortSignal,
    capture?: BrowserCapture,
    instructions?: string,
  ) {
    validateConfig(this.config, intent);
    if (["capture", "art", "idea"].includes(intent)) {
      const localImage =
        intent === "capture" && isLocalSource(input) && capture?.url === input;
      if (localImage) capture = validateBrowserCapture(capture);
      const url = new URL(input);
      if (
        (!localImage && !["http:", "https:"].includes(url.protocol)) ||
        url.username ||
        url.password
      )
        throw Error("Provide an HTTP(S) URL");
    }
    if (!this.fixture && intent !== "probe") {
      const status = await this.usage.status();
      const pause = status.providers[this.config.provider]?.paused;
      if (pause)
        return {
          input,
          exitCode: 1,
          outcome: { status: "paused", reason: pause, sourceUrl: input },
        };
    }
    const effectiveInstructions =
      instructions ??
      capture?.instructions ??
      (intent === "capture"
        ? (await this.vault.sourceRecords(input))[0]?.item.instructions
        : undefined);
    const result = await runJob({
      vault: this.vault,
      config: { ...this.config, stateDir: this.stateDir },
      intent,
      input,
      signal,
      browserCapture: capture,
      focus: effectiveInstructions,
      instructions: effectiveInstructions,
      batchId: randomUUID(),
      ...(this.fixture ? await fixtures(this.fixtureImage) : {}),
    });
    const status = await this.status();
    const allowance = status.usage.providers[status.provider]?.allowance;
    const warnings = [
      ...(status.usage.warning
        ? [
            "Approaching GG application usage limit; run gg usage for reset times.",
          ]
        : []),
      ...(allowance && allowance.usedPercent >= 80
        ? [
            `${status.provider} allowance last reported ${allowance.usedPercent}% used at ${new Date(allowance.observedAt).toISOString()}.`,
          ]
        : []),
    ];
    return {
      ...result,
      usageWarnings: warnings,
      provider: status.provider,
      subscriptionAllowance: allowance ?? "unavailable",
    };
  }
  async status() {
    return {
      provider: this.config.provider,
      model: this.config.model,
      usage: await this.usage.status(),
    };
  }
  async execute(input: Command, signal?: AbortSignal): Promise<unknown> {
    if (
      !input ||
      typeof input.command !== "string" ||
      JSON.stringify(input).length > 100000
    )
      throw Error("Invalid command");
    const text = input.text ?? "";
    if (typeof text !== "string" || text.length > 20000)
      throw Error("Command text is too long");
    switch (input.command) {
      case "search":
        return this.vault.search(text);
      case "ask":
      case "do":
        if (!text.trim()) throw Error("Provide instructions");
        return this.run(
          input.command === "ask" ? "ask" : "manage",
          text,
          signal,
        );
      case "capture": {
        const url = new URL(text);
        if (
          !["http:", "https:"].includes(url.protocol) ||
          url.username ||
          url.password
        )
          throw Error("Provide an HTTP(S) URL");
        return this.run("capture", text, signal, undefined, input.instructions);
      }
      case "confirm":
        if (!input.id) throw Error("Provide a proposal ID");
        return this.vault.confirm(input.id);
      case "undo":
        return this.vault.undo(input.id);
      case "history":
        return this.vault.history();
      case "list":
        return (await this.vault.items()).map((v) => ({
          id: v.item.id,
          title: v.item.title,
          subvault: v.item.subvault,
          path: v.path,
        }));
      case "usage":
      case "status":
        return this.status();
      case "resume":
        await this.usage.resume(this.config.provider);
        return this.status();
      case "override":
        await this.usage.override(
          input.requests!,
          input.tokens!,
          input.minutes ?? 60,
        );
        return this.status();
      case "provider": {
        if (!input.provider || !input.model)
          throw Error("Specify provider and model explicitly");
        const next = {
          ...this.config,
          provider: input.provider,
          model: input.model,
          api:
            input.provider === "openrouter"
              ? ("openai-completions" as const)
              : ("openai-responses" as const),
        };
        delete next.apiKey;
        validateConfig(next, "capture");
        Object.assign(this.config, next);
        delete this.config.apiKey;
        await writeJson(join(this.stateDir, "settings.json"), {
          ...next,
          vault: this.vault.root,
        });
        return this.status();
      }
      default:
        throw Error("Unknown remote command");
    }
  }
}
