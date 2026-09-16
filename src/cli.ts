import { parseArgs } from "node:util";
import { readFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createInterface } from "node:readline/promises";
import { stdin, stdout } from "node:process";
import { Vault } from "./vault.ts";
import { Controller, type Command } from "./controller.ts";
import { Devices } from "./devices.ts";
import { createReceiver } from "./receiver.ts";
import { validateBrowserCapture } from "./browser-capture.ts";
import { defaultStateDir, readJson, writeJson } from "./state.ts";
import { providerRuntime, loginProvider } from "./model-service.ts";
import { connect, connection, remote } from "./client.ts";
import type { Config } from "./types.ts";

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    vault: { type: "string" },
    config: { type: "string" },
    model: { type: "string" },
    provider: { type: "string" },
    api: { type: "string" },
    fixture: { type: "boolean" },
    stdin: { type: "boolean" },
    focus: { type: "string" },
    instructions: { type: "string" },
    seconds: { type: "string" },
    "max-requests": { type: "string" },
    "fixture-image": { type: "string" },
    json: { type: "boolean" },
    port: { type: "string" },
    "state-dir": { type: "string" },
    "public-origin": { type: "string" },
    local: { type: "boolean" },
    scope: { type: "string" },
    tokens: { type: "string" },
    requests: { type: "string" },
    minutes: { type: "string" },
  },
});
const command = positionals.shift() ?? "help";
const stateDir = resolve(values["state-dir"] ?? defaultStateDir());
const help = `GG — your personal archive

  gg init [subvaults...] --vault PATH
  gg configure --vault PATH --provider openai|openrouter|openai-codex --model SLUG
  gg login [PROVIDER] | logout [PROVIDER] | models [PROVIDER]
  gg serve [--public-origin https://gg.example.com] [--port 48123]
  gg pair [--scope capture|manage] | devices | revoke DEVICE_ID
  gg connect SERVER_URL                  (prompts for a management pairing code)
  gg capture URL... [--instructions TEXT] [--stdin]
  gg capture-file FILE...                (browser snapshots)
  gg ask QUESTION | search QUERY | do INSTRUCTIONS | chat
  gg list | history | undo [OPERATION_OR_BATCH] | confirm PROPOSAL
  gg usage | resume | override --requests N --tokens N [--minutes N]
  gg provider PROVIDER MODEL             (explicit switch; never automatic)
  gg index | queue | probe

--vault PATH --config FILE --state-dir PATH --local --json
Legacy art|painting|idea URL commands remain available.
A connected CLI uses the server for archive commands. --local uses local settings.
`;
async function question(message: string) {
  const rl = createInterface({ input: stdin, output: stdout });
  try {
    return await rl.question(message);
  } finally {
    rl.close();
  }
}
async function secret(message: string): Promise<string> {
  if (!stdin.isTTY) return question(message);
  stdout.write(message);
  stdin.setRawMode(true);
  stdin.resume();
  return new Promise((resolve, reject) => {
    let value = "";
    const finish = () => {
      stdin.off("data", onData);
      stdin.setRawMode(false);
      stdin.pause();
      stdout.write("\n");
    };
    const onData = (chunk: Buffer) => {
      for (const ch of chunk.toString()) {
        if (ch === "\u0003") {
          finish();
          reject(Error("Cancelled"));
          return;
        }
        if (ch === "\r" || ch === "\n") {
          finish();
          resolve(value);
          return;
        }
        if (ch === "\u007f") {
          value = value.slice(0, -1);
        } else value += ch;
      }
    };
    stdin.on("data", onData);
  });
}
function print(result: unknown) {
  console.log(
    typeof result === "string" ? result : JSON.stringify(result, null, 2),
  );
}
async function main() {
  if (command === "help") {
    print(help);
    return;
  }
  const persisted = await readJson<
    Partial<Config> & { vault?: string; publicOrigin?: string }
  >(join(stateDir, "settings.json"), {});
  const file = values.config
    ? JSON.parse(await readFile(values.config, "utf8"))
    : {};
  const config: Config = {
    provider: "openai",
    model: "gpt-5.6-luna",
    ...persisted,
    ...file,
    stateDir,
  };
  if (values.provider) config.provider = values.provider as Config["provider"];
  if (values.model) config.model = values.model;
  config.api =
    (values.api as Config["api"]) ??
    file.api ??
    (config.provider === "openrouter"
      ? "openai-completions"
      : "openai-responses");
  if (values.seconds) config.maxSeconds = Number(values.seconds);
  if (values["max-requests"])
    config.maxRequests = Number(values["max-requests"]);
  const envKey =
    config.provider === "openai"
      ? process.env.OPENAI_API_KEY
      : config.provider === "openrouter"
        ? process.env.OPENROUTER_API_KEY
        : undefined;
  config.apiKey = envKey ?? config.apiKey;
  const vaultPath = resolve(
    values.vault ?? file.vault ?? persisted.vault ?? ".runs/vault",
  );
  if (command === "init") {
    const vault = await Vault.create(
      vaultPath,
      positionals.length
        ? positionals
        : ["Paintings", "Photography", "Sculptures", "Ideas", "Inbox"],
    );
    await writeJson(join(stateDir, "settings.json"), {
      ...persisted,
      vault: vault.root,
    });
    print({ vault: vault.root, subvaults: vault.areas });
    return;
  }
  if (command === "configure") {
    const { validateConfig } = await import("./config.ts");
    validateConfig(config, "capture");
    const { apiKey, ...settings } = config;
    await writeJson(join(stateDir, "settings.json"), {
      ...settings,
      vault: vaultPath,
      publicOrigin: values["public-origin"] ?? persisted.publicOrigin,
    });
    print(
      "GG settings saved. Use gg login or provider environment variables for credentials.",
    );
    return;
  }
  if (["login", "logout", "models"].includes(command)) {
    const provider = positionals[0] ?? config.provider;
    if (!["openai", "openrouter", "openai-codex"].includes(provider))
      throw Error("Unsupported provider");
    if (command === "login") {
      await loginProvider(stateDir, provider, {
        prompt: async (p) => {
          if (p.type === "select") {
            print(p.options.map((o) => `${o.id}: ${o.label}`).join("\n"));
            return question(p.message + ": ");
          }
          return p.type === "secret"
            ? secret(p.message + ": ")
            : question(p.message + ": ");
        },
        notify: (event) => {
          if (event.type === "auth_url")
            print(
              `${event.instructions ?? "Open this sign-in link:"}\n${event.url}`,
            );
          else if (event.type === "device_code")
            print(`${event.verificationUri}\nCode: ${event.userCode}`);
          else print(event.message);
        },
      });
      print(
        "Signed in. Select the provider explicitly with gg provider PROVIDER MODEL or gg configure.",
      );
      return;
    }
    const runtime = await providerRuntime(stateDir);
    if (command === "logout") {
      await runtime.logout(provider);
      print("Signed out.");
      return;
    }
    await runtime.refresh({ allowNetwork: true, providers: [provider] });
    print(
      runtime.getModels(provider).map((m) => ({ id: m.id, input: m.input })),
    );
    return;
  }
  const devices = new Devices(stateDir);
  if (command === "pair") {
    print({
      code: await devices.pairing(
        (values.scope ?? "capture") as "capture" | "manage",
      ),
      expiresInMinutes: 10,
      scope: values.scope ?? "capture",
    });
    return;
  }
  if (command === "devices") {
    print(await devices.list());
    return;
  }
  if (command === "revoke") {
    await devices.revoke(positionals[0]);
    print("Device revoked.");
    return;
  }
  if (command === "connect") {
    print(
      await connect(
        stateDir,
        positionals[0],
        await secret("Management pairing code: "),
      ),
    );
    return;
  }
  const client = values.local ? undefined : await connection(stateDir);
  let controller: Controller | undefined;
  const local = async () =>
    (controller ??= new Controller(
      await Vault.open(vaultPath),
      config,
      stateDir,
      values.fixture,
      values["fixture-image"],
    ));
  const execute = async (input: Command) =>
    client ? remote(client, input) : (await local()).execute(input);
  if (command === "serve") {
    const c = await local();
    const port = Number(values.port ?? 48123);
    if (!Number.isSafeInteger(port) || port < 1 || port > 65535)
      throw Error("Invalid port");
    const publicOrigin = values["public-origin"] ?? persisted.publicOrigin;
    const receiver = await createReceiver({
      vault: c.vault,
      port,
      devices,
      publicOrigin,
      usage: () => c.status(),
      command: (input, signal) => c.execute(input, signal),
      processCapture: (capture, signal) =>
        c.run(
          capture.intent,
          capture.url,
          signal,
          capture,
          capture.instructions ?? capture.focus,
        ),
    });
    print(
      `GG receiver: ${publicOrigin ?? receiver.url}\nBackend: ${receiver.url}\nPairing code (10 minutes): ${receiver.token}\nUse gg pair --scope manage for a CLI device.`,
    );
    await new Promise<void>((resolve) => {
      process.once("SIGINT", resolve);
      process.once("SIGTERM", resolve);
    });
    await receiver.close();
    return;
  }
  if (command === "chat") {
    print(
      "GG vault chat. /exit to leave; /confirm ID and /undo [ID] operate on returned previews/history.",
    );
    const rl = createInterface({ input: stdin, output: stdout });
    const conversation: string[] = [];
    try {
      while (true) {
        const text = (await rl.question("gg> ")).trim();
        if (text === "/exit") break;
        if (!text) continue;
        const input: Command = text.startsWith("/confirm ")
          ? { command: "confirm", id: text.slice(9).trim() }
          : text.startsWith("/undo")
            ? { command: "undo", id: text.slice(5).trim() || undefined }
            : {
                command: "do",
                text: conversation.length
                  ? `Previous conversation (context only):\n${conversation.join("\n")}\n\nCurrent instruction: ${text}`
                  : text,
              };
        try {
          const result = await execute(input);
          print(result);
          conversation.push(
            `User: ${text}`,
            `GG: ${JSON.stringify(result).slice(0, 3000)}`,
          );
          while (conversation.join("\n").length > 12000)
            conversation.splice(0, 2);
        } catch (e) {
          console.error(String(e));
        }
      }
    } finally {
      rl.close();
    }
    return;
  }
  if (command === "capture-file") {
    if (!positionals.length) throw Error("Provide a snapshot file");
    if (client)
      throw Error(
        "Send browser snapshots through the extension; capture-file is a local administrative command",
      );
    for (const file of positionals) {
      const capture = validateBrowserCapture(
        JSON.parse(await readFile(file, "utf8")),
      );
      const result = await (
        await local()
      ).run(
        capture.intent,
        capture.url,
        undefined,
        capture,
        capture.instructions ?? capture.focus,
      );
      print(result);
      if (
        result.exitCode ||
        ["failed", "partial", "paused"].includes(result.outcome?.status ?? "")
      )
        process.exitCode = 1;
    }
    return;
  }
  if (["capture", "art", "painting", "idea"].includes(command)) {
    const urls = [...positionals];
    if (values.stdin) {
      let input = "";
      for await (const c of stdin) input += c;
      urls.push(
        ...input
          .split(/\r?\n/)
          .map((v) => v.trim())
          .filter(Boolean),
      );
    }
    if (!urls.length) throw Error("Provide a URL");
    for (const url of urls) {
      try {
        const result: any =
          command === "capture"
            ? await execute({
                command: "capture",
                text: url,
                instructions: values.instructions ?? values.focus,
              })
            : await (
                await local()
              ).run(
                command === "painting" ? "art" : (command as "art" | "idea"),
                url,
                undefined,
                undefined,
                values.instructions ?? values.focus,
              );
        print(result);
        if (
          result.exitCode ||
          ["failed", "partial", "paused"].includes(result.outcome?.status)
        )
          process.exitCode = 1;
      } catch (e) {
        print({ input: url, error: String(e) });
        process.exitCode = 1;
      }
    }
    return;
  }
  if (command === "probe") {
    const result = await (await local()).run("probe", "probe");
    print(result);
    if (result.exitCode) process.exitCode = 1;
    return;
  }
  if (command === "index") {
    print(await (await local()).vault.index());
    return;
  }
  if (command === "queue") {
    const { readdir } = await import("node:fs/promises");
    print(
      await Promise.all(
        (await readdir(join(vaultPath, "queue"))).map(async (p) =>
          JSON.parse(await readFile(join(vaultPath, "queue", p), "utf8")),
        ),
      ),
    );
    return;
  }
  const result: any = await execute({
    command,
    text: positionals.join(" "),
    id: positionals[0],
    provider:
      command === "provider"
        ? (positionals[0] as Config["provider"])
        : undefined,
    model: command === "provider" ? positionals[1] : undefined,
    requests: Number(values.requests),
    tokens: Number(values.tokens),
    minutes: values.minutes ? Number(values.minutes) : undefined,
  });
  print(result);
  if (
    result?.exitCode ||
    ["failed", "partial", "paused"].includes(result?.outcome?.status)
  )
    process.exitCode = 1;
}
main().catch((e) => {
  console.error(e instanceof Error ? e.message : String(e));
  process.exitCode = 1;
});
