import http from "node:http";
import { mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Vault } from "../src/vault.ts";
import { createGateway } from "../src/gateway.ts";

const model = process.env.GG_MODEL ?? "",
  apiKey = process.env.OPENAI_API_KEY;
if (!model || !apiKey) throw Error("Set GG_MODEL and OPENAI_API_KEY");
const body = {
  model,
  input: "Reply with the word OK.",
  stream: true,
  max_output_tokens: 256,
  store: false,
};
const vault = await Vault.create(
  await mkdtemp(join(tmpdir(), "gg-provider-diagnostic-")),
  ["Ideas"],
);
const gateway = await createGateway({
  vault,
  intent: "ask",
  input: "diagnostic",
  config: { provider: "openai", model, apiKey, maxSeconds: 30 },
});
async function direct() {
  const start = Date.now();
  try {
    const r = await fetch("https://api.openai.com/v1/responses", {
      method: "POST",
      headers: {
        "content-type": "application/json",
        authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(30000),
    });
    let bytes = 0;
    if (r.body) for await (const c of r.body) bytes += c.length;
    return {
      path: "direct",
      status: r.status,
      bytes,
      seconds: (Date.now() - start) / 1000,
    };
  } catch {
    return {
      path: "direct",
      error: "Timed out or transport failure",
      seconds: (Date.now() - start) / 1000,
    };
  }
}
async function proxied() {
  const start = Date.now();
  return new Promise((resolve) => {
    const req = http.request(
      {
        socketPath: gateway.socket,
        path: "/model/responses",
        method: "POST",
        headers: {
          authorization: `Bearer ${gateway.token}`,
          "content-type": "application/json",
        },
      },
      (res) => {
        let bytes = 0;
        res.on("data", (c) => (bytes += c.length));
        res.on("end", () =>
          resolve({
            path: "gateway",
            status: res.statusCode,
            bytes,
            seconds: (Date.now() - start) / 1000,
          }),
        );
        res.on("error", () =>
          resolve({
            path: "gateway",
            error: "Stream interrupted",
            seconds: (Date.now() - start) / 1000,
          }),
        );
      },
    );
    req.on("error", () =>
      resolve({
        path: "gateway",
        error: "Transport failure",
        seconds: (Date.now() - start) / 1000,
      }),
    );
    req.end(JSON.stringify(body));
  });
}
try {
  console.log(JSON.stringify(await Promise.all([direct(), proxied()])));
} finally {
  await gateway.close();
}
