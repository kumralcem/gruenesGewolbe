import { mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { Vault } from "../src/vault.ts";
import { runJob } from "../src/runner.ts";
import { seedIdeas } from "./sample-vault.ts";
import type { Config, Intent } from "../src/types.ts";

const provider = (process.env.GG_PROVIDER ?? "openai") as Config["provider"];
const config: Config = {
  provider,
  model: process.env.GG_MODEL ?? "",
  api: provider === "openai" ? "openai-responses" : "openai-completions",
  apiKey:
    process.env[
      provider === "openai" ? "OPENAI_API_KEY" : "OPENROUTER_API_KEY"
    ],
  maxRequests: 10,
  maxSeconds: 150,
};
if (!config.apiKey || !config.model)
  throw Error(
    "Set GG_MODEL and the selected provider API key; optionally GG_PROVIDER",
  );
const root = resolve(".runs", `live-${Date.now()}`);
await mkdir(root, { recursive: true });
const vault = await Vault.create(join(root, "vault"), [
  "Paintings",
  "Photography",
  "Sculptures",
  "Ideas",
]);
await seedIdeas(vault);
const cases: { intent: Intent; input: string }[] = [
  {
    intent: "art",
    input:
      "https://commons.wikimedia.org/wiki/File:Le_Chevalier_aux_Fleurs_1894_Georges_Rochegrosse_1859_1938.jpg",
  },
  {
    intent: "art",
    input:
      "https://fr.wikipedia.org/wiki/Tancr%C3%A8de_Bastet#/media/Fichier:L_atelier_de_Cabanel_a_l_ecole_des_Beaux_Arts.jpg",
  },
  {
    intent: "art",
    input: "https://x.com/solisolsoli/status/2092378489093595354",
  },
  {
    intent: "art",
    input: "https://x.com/EvolveWildlife/status/2097333486659260893",
  },
  {
    intent: "art",
    input: "https://x.com/Noldorcitizen/status/2097248441860497708",
  },
  { intent: "idea", input: "https://www.youtube.com/watch?v=xJaMTo2YgO8" },
  { intent: "idea", input: "https://en.wikipedia.org/wiki/Inbox_Zero" },
  {
    intent: "ask",
    input:
      "Bring me the files that touch on managing customers using email management agents",
  },
  {
    intent: "ask",
    input: "Find instructions for repairing a nuclear reactor coolant pump",
  },
];
const reports = [];
for (const entry of cases) {
  console.log(`Starting ${entry.intent}: ${entry.input}`);
  const start = Date.now();
  const result = await runJob({
    vault,
    config,
    ...entry,
    onEvent: (event) => {
      if (["tool", "outcome", "error"].includes(event.type))
        console.log(JSON.stringify(event));
    },
  });
  const report = { ...result, durationSeconds: (Date.now() - start) / 1000 };
  reports.push(report);
  await writeFile(join(root, "results.json"), JSON.stringify(reports, null, 2));
  console.log(
    JSON.stringify({
      input: entry.input,
      outcome: result.outcome,
      answers: result.answers,
      durationSeconds: report.durationSeconds,
    }),
  );
}
console.log(`Report: ${join(root, "results.json")}`);
