import { test } from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import http from "node:http";
import { mkdtemp, readFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import sharp from "sharp";
import { createReceiver } from "../src/receiver.ts";
import { Vault } from "../src/vault.ts";

test(
  "real Chromium extension captures an authenticated page and exact authenticated image without exporting credentials",
  { timeout: 60000 },
  async () => {
    const root = await mkdtemp(join(tmpdir(), "gg-extension-test-"));
    const image = await sharp({
      create: { width: 300, height: 200, channels: 3, background: "#236a61" },
    })
      .png()
      .toBuffer();
    const website = http.createServer((req, res) => {
      if (req.headers.cookie !== "session=COOKIE_SENTINEL") {
        res.writeHead(401);
        res.end("Sign in required");
        return;
      }
      if (req.url === "/image.png") {
        res.writeHead(200, { "content-type": "image/png" });
        res.end(image);
        return;
      }
      res.writeHead(200, { "content-type": "text/html" });
      res.end(
        '<title>Signed-in painting</title><article><h1>Member instructions</h1><p>Export your inbox, classify messages, and review drafts.</p><img src="/image.png" alt="The selected painting"><form><input type="password" value="PASSWORD_SENTINEL"><textarea>PRIVATE_FORM_TEXT</textarea></form><div style="display:none"><span>HIDDEN_SENTINEL</span></div><script>window.secret="SCRIPT_SENTINEL"</script></article>',
      );
    });
    await new Promise<void>((r) => website.listen(0, "127.0.0.1", r));
    const origin = `http://127.0.0.1:${(website.address() as any).port}`;
    const vault = await Vault.create(join(root, "vault"), ["Ideas"]);
    const captures: any[] = [];
    const receiver = await createReceiver({
      vault,
      port: 0,
      processCapture: async (c) => {
        captures.push(c);
        return {
          outcome: {
            status: "skipped",
            reason: "Browser integration fixture received",
          },
        };
      },
    });
    let context:
      Awaited<ReturnType<typeof chromium.launchPersistentContext>> | undefined;
    try {
      context = await chromium.launchPersistentContext(join(root, "profile"), {
        channel: "chromium",
        headless: true,
        args: [
          `--disable-extensions-except=${resolve("extension")}`,
          `--load-extension=${resolve("extension")}`,
        ],
      });
      await context.addCookies([
        {
          name: "session",
          value: "COOKIE_SENTINEL",
          url: origin,
          httpOnly: true,
          sameSite: "Lax",
        },
      ]);
      const page = await context.newPage();
      await page.goto(origin);
      await page.waitForFunction(() => document.images[0]?.naturalWidth > 0);
      const worker =
        context.serviceWorkers()[0] ??
        (await context.waitForEvent("serviceworker"));
      await worker.evaluate(
        async (settings) => {
          await (globalThis as any).chrome.storage.local.set(settings);
        },
        { receiver: receiver.url, token: receiver.token },
      );
      const tab = await worker.evaluate(async (origin) => {
        return (await (globalThis as any).chrome.tabs.query({})).find(
          (tab: any) => tab.url?.startsWith(origin),
        );
      }, origin);
      const capturePagePromise = context.waitForEvent("page");
      await worker.evaluate(async (tab) => {
        await (globalThis as any).startCapture(tab);
      }, tab);
      const ui = await capturePagePromise;
      await ui.getByRole("button", { name: "Send to my vault" }).click();
      await ui
        .getByRole("status")
        .filter({ hasText: "Capture accepted" })
        .waitFor();
      await receiver.idle();
      assert.equal(captures[0].intent, "idea");
      assert.match(captures[0].text, /classify messages/);
      assert.doesNotMatch(
        JSON.stringify(captures[0]),
        /COOKIE_SENTINEL|PASSWORD_SENTINEL|PRIVATE_FORM_TEXT|SCRIPT_SENTINEL|HIDDEN_SENTINEL/,
      );
      const secondPromise = context.waitForEvent("page");
      await worker.evaluate(async (tab) => {
        await (globalThis as any).startCapture(tab);
      }, tab);
      const second = await secondPromise;
      await second.getByLabel("One image", { exact: true }).check();
      await second.getByRole("button", { name: "Send to my vault" }).click();
      await second
        .getByRole("status")
        .filter({ hasText: "Capture accepted" })
        .waitFor();
      await receiver.idle();
      assert.equal(captures[1].intent, "art");
      assert.deepEqual(Buffer.from(captures[1].image.bytes, "base64"), image);
      await mkdir(".runs/extension-evidence", { recursive: true });
      await second.screenshot({
        path: ".runs/extension-evidence/capture.png",
        fullPage: true,
      });
      const manifest = JSON.parse(
        await readFile("extension/manifest.json", "utf8"),
      );
      assert.ok(!manifest.permissions.includes("cookies"));
      assert.deepEqual(manifest.host_permissions, ["http://127.0.0.1/*"]);
    } finally {
      await context?.close();
      await receiver.close();
      await new Promise<void>((r) => website.close(() => r()));
    }
  },
);
