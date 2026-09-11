import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { Vault } from "../src/vault.ts";
import { runJob } from "../src/runner.ts";
import { fixtures } from "../src/fixtures.ts";
import { seedIdeas } from "./sample-vault.ts";

const root = resolve(".runs", `demo-${Date.now()}`);
await mkdir(root, { recursive: true });
const vault = await Vault.create(root, [
  "Paintings",
  "Photography",
  "Sculptures",
  "Ideas",
]);
await seedIdeas(vault);
const config = {
  provider: "openai" as const,
  model: "fixture-model",
  api: "openai-completions" as const,
  maxSeconds: 40,
};
const fake = await fixtures();
const reports = [];
for (const path of ["blocked", "ambiguous", "art", "art"]) {
  const report = await runJob({
    vault,
    config,
    intent: "art",
    input: `https://fixtures.example/${path}`,
    ...fake,
  });
  reports.push(report);
  console.log(path, report.outcome);
}
const ask = await runJob({
  vault,
  config,
  intent: "ask",
  input: "managing customers using email agents",
  ...fake,
});
reports.push(ask);
console.log("ask", ask.answers);
await writeFile(
  resolve(root, "demo-results.json"),
  JSON.stringify(reports, null, 2),
);
console.log(
  `Demo vault: ${root}\nThis uses real Pi with a deterministic provider stub; it is not a live-model quality evaluation.`,
);
if (reports.some((r) => r.exitCode !== 0 || r.outcome?.status === "failed"))
  process.exitCode = 1;
