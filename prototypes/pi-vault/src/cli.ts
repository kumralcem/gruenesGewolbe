import { parseArgs } from "node:util";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { Vault } from "./vault.ts";
import { runJob, JobCancelledError } from "./runner.ts";
import { validateConfig } from "./config.ts";
import { fixtures } from "./fixtures.ts";
import type { Config, Intent } from "./types.ts";

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    vault: { type: "string", default: ".runs/vault" },
    config: { type: "string" },
    model: { type: "string" },
    provider: { type: "string" },
    api: { type: "string" },
    fixture: { type: "boolean" },
    stdin: { type: "boolean" },
    focus: { type: "string" },
    seconds: { type: "string" },
    "max-requests": { type: "string" },
    "fixture-image": { type: "string" },
    json: { type: "boolean" },
  },
});
const command = positionals.shift();
try {
  if (command === "init") {
    const vault = await Vault.create(
      resolve(values.vault),
      positionals.length
        ? positionals
        : ["Paintings", "Photography", "Sculptures", "Ideas"],
    );
    console.log(JSON.stringify({ vault: vault.root, areas: vault.areas }));
  } else if (!command || command === "help")
    console.log(
      "PROTOTYPE — disposable vaults only.\ninit [areas...] | art|painting|idea URL... [--stdin] | search QUERY | ask QUERY | probe | queue\n--vault PATH --provider openai|openrouter --model SLUG --config PATH --fixture --focus TEXT --json",
    );
  else {
    const vault = await Vault.open(resolve(values.vault));
    if (command === "search")
      console.log(
        JSON.stringify(await vault.search(positionals.join(" ")), null, 2),
      );
    else if (command === "queue")
      console.log(
        JSON.stringify(
          await Promise.all(
            (await readdir(`${vault.root}/queue`)).map((p) =>
              readFile(`${vault.root}/queue/${p}`, "utf8").then(JSON.parse),
            ),
          ),
          null,
          2,
        ),
      );
    else {
      const config: Config = values.config
        ? JSON.parse(await readFile(values.config, "utf8"))
        : { provider: "openai", model: "" };
      if (values.provider)
        config.provider = values.provider as Config["provider"];
      if (values.model) config.model = values.model;
      if (values.api) config.api = values.api as Config["api"];
      config.api ??=
        config.provider === "openai"
          ? "openai-responses"
          : "openai-completions";
      if (values.fixture) {
        config.model ||= "fixture-model";
        config.api = "openai-completions";
      }
      if (!["openai", "openrouter"].includes(config.provider))
        throw Error("Provider must be openai or openrouter");
      config.apiKey =
        process.env[
          config.provider === "openai" ? "OPENAI_API_KEY" : "OPENROUTER_API_KEY"
        ] ?? config.apiKey;
      config.maxSeconds = Number(values.seconds ?? config.maxSeconds ?? 180);
      config.maxRequests = Number(
        values["max-requests"] ?? config.maxRequests ?? 10,
      );
      if (!config.model && command !== "probe")
        throw Error("Set --model to a provider model slug");
      if (!values.fixture && command !== "probe" && !config.apiKey)
        throw Error(
          "Configure the provider API key in the controller environment or a private config file",
        );
      let inputs = command === "ask" ? [positionals.join(" ")] : positionals;
      if (values.stdin) {
        if (!["art", "painting", "idea"].includes(command))
          throw Error("--stdin is for capture commands");
        let s = "";
        for await (const c of process.stdin) s += c;
        inputs.push(
          ...s
            .split(/\r?\n/)
            .map((s) => s.trim())
            .filter(Boolean),
        );
      }
      if (command === "probe") inputs = ["probe"];
      const intent = (command === "painting" ? "art" : command) as Intent;
      if (
        !["art", "idea", "ask", "probe"].includes(intent) ||
        !inputs.length ||
        inputs.some((v) => !v.trim())
      )
        throw Error("Provide a valid command and input");
      validateConfig(config, intent);
      const fake = values.fixture
        ? await fixtures(values["fixture-image"])
        : {};
      for (const input of inputs) {
        try {
          if (intent === "art" || intent === "idea") {
            const url = new URL(input);
            if (
              !["http:", "https:"].includes(url.protocol) ||
              url.username ||
              url.password
            )
              throw Error(
                "Capture requires a public HTTP(S) URL without credentials",
              );
          }
          const r = await runJob({
            vault,
            config,
            intent,
            input,
            focus: values.focus,
            ...fake,
            onEvent: (e) => {
              if (
                !values.json &&
                ["tool", "outcome", "results", "probe", "error"].includes(
                  e.type,
                )
              )
                console.log(JSON.stringify(e));
            },
          });
          console.log(
            JSON.stringify(
              values.json
                ? r
                : {
                    input: r.input,
                    ...(r.outcome ?? { status: "completed" }),
                    ...(intent === "ask" ? { answers: r.answers } : {}),
                    modelRequests: r.modelRequests,
                  },
            ),
          );
          if (r.exitCode || r.outcome?.status === "failed")
            process.exitCode = 1;
        } catch (e) {
          if (e instanceof JobCancelledError) throw e;
          console.log(
            JSON.stringify({
              input,
              outcome: {
                status: "failed",
                reason: e instanceof Error ? e.message : String(e),
              },
            }),
          );
          process.exitCode = 1;
        }
      }
    }
    if (vault.problems.size)
      console.error(
        JSON.stringify({
          vaultProblems: Array.from(vault.problems, ([path, reason]) => ({
            path,
            reason,
          })),
        }),
      );
  }
} catch (e) {
  console.error(e instanceof Error ? e.message : String(e));
  process.exitCode = 1;
}
