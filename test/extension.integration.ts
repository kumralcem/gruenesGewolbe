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
      if (req.url === "/selection") {
        res.end(
          '<main><p id="public">Public introduction.</p><form><article><p id="draft">PRIVATE_DRAFT_SENTINEL</p></article></form></main>',
        );
        return;
      }
      if (req.url === "/editor" || req.url === "/form") {
        const article =
          "<article" +
          (req.url === "/editor" ? " contenteditable" : "") +
          '><p>PRIVATE_DRAFT_SENTINEL</p><img src="/image.png"></article>';
        res.end(req.url === "/form" ? "<form>" + article + "</form>" : article);
        return;
      }
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
      await ui.getByRole("button", { name: "Capture", exact: true }).click();
      await ui
        .getByRole("status")
        .filter({ hasText: "Capture received" })
        .waitFor();
      await receiver.idle();
      assert.equal(captures[0].intent, "capture");
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
      await second
        .getByLabel("Instructions (optional)")
        .fill("Save each image separately with its surrounding context.");
      await second
        .getByRole("button", { name: "Capture", exact: true })
        .click();
      await second
        .getByRole("status")
        .filter({ hasText: "Capture received" })
        .waitFor();
      await receiver.idle();
      assert.equal(captures[1].intent, "capture");
      assert.match(captures[1].instructions, /each image separately/);
      assert.deepEqual(
        Buffer.from(captures[1].images[0].bytes, "base64"),
        image,
      );
      await mkdir(".runs/extension-evidence", { recursive: true });
      await second.screenshot({
        path: ".runs/extension-evidence/capture.png",
        fullPage: true,
      });
      // Load the action popup document while the source remains the active tab.
      const popup = await context.newPage();
      await page.bringToFront();
      const extensionOrigin = ui.url().split("/capture.html")[0];
      await popup.goto(extensionOrigin + "/capture.html?popup=1");
      await popup
        .getByLabel("Instructions (optional)")
        .fill("Keep the popup capture.");
      const beforeEnter = captures.length;
      await popup.getByLabel("Instructions (optional)").press("Shift+Enter");
      assert.equal(captures.length, beforeEnter);
      assert.match(
        await popup.getByLabel("Instructions (optional)").inputValue(),
        /\n/,
      );
      await popup.getByLabel("Instructions (optional)").press("Enter");
      await popup
        .locator("#status")
        .filter({ hasText: "Capture received" })
        .waitFor();
      await receiver.idle();
      assert.equal(captures[2].instructions.trim(), "Keep the popup capture.");
      assert.deepEqual(
        Buffer.from(captures[2].images[0].bytes, "base64"),
        image,
      );
      assert.equal(await popup.locator("#settings").isVisible(), false);
      assert.equal(await popup.locator("#version").textContent(), "GG 0.4.1");
      assert.equal(await popup.locator("#source").isVisible(), false);
      await popup.setViewportSize({ width: 400, height: 440 });
      const dashboard = await context.newPage();
      await dashboard.goto(extensionOrigin + "/capture.html");
      assert.equal(
        await dashboard.locator("#capture-panel").isVisible(),
        false,
      );
      assert.equal(await dashboard.locator("#dashboard").isVisible(), false);
      assert.equal(await dashboard.locator("#settings").isVisible(), true);
      assert.equal(
        await popup.locator("html").getAttribute("data-theme"),
        "dark",
      );
      await dashboard.locator("#capture-rules:not([disabled])").waitFor();
      assert.match(
        await dashboard.locator("#capture-rules").inputValue(),
        /include every substantive item/,
      );
      await dashboard
        .locator("#capture-rules")
        .fill("List all five tips with practical examples.");
      await dashboard
        .getByRole("button", { name: "Save instructions", exact: true })
        .click();
      await dashboard
        .locator("#rules-status")
        .filter({ hasText: "Saved." })
        .waitFor();
      assert.equal(
        (await vault.captureRules()).text,
        "List all five tips with practical examples.",
      );
      await dashboard.reload();
      await dashboard.locator("#capture-rules:not([disabled])").waitFor();
      assert.equal(
        await dashboard.locator("#capture-rules").inputValue(),
        "List all five tips with practical examples.",
      );
      assert.equal(
        await popup.locator("#capture-rules-panel").isVisible(),
        false,
      );
      await dashboard.locator("#theme").selectOption("light");
      await popup.waitForFunction(
        () => document.documentElement.dataset.theme === "light",
      );
      await popup.emulateMedia({ colorScheme: "dark" });
      await dashboard.locator("#theme").selectOption("system");
      await popup.waitForFunction(
        () => document.documentElement.dataset.theme === "dark",
      );
      await popup.emulateMedia({ colorScheme: "light" });
      await popup.waitForFunction(
        () => document.documentElement.dataset.theme === "light",
      );
      await dashboard.locator("#theme").selectOption("dark");
      await popup.waitForFunction(
        () => document.documentElement.dataset.theme === "dark",
      );
      await popup.screenshot({ path: ".runs/extension-evidence/popup.png" });
      for (const path of ["/editor", "/form", "/selection"]) {
        await page.goto(origin + path);
        if (path === "/selection")
          await page.evaluate(() => {
            const range = document.createRange();
            range.setStart(document.querySelector("#public")!.firstChild!, 0);
            range.setEnd(
              document.querySelector("#draft")!.firstChild!,
              "PRIVATE_DRAFT_SENTINEL".length,
            );
            getSelection()!.removeAllRanges();
            getSelection()!.addRange(range);
          });
        const privateSnapshot = await worker.evaluate(
          async (tab) => {
            await (globalThis as any).startCapture(tab);
            const entries = Object.values(
              await (globalThis as any).chrome.storage.session.get(null),
            ) as any[];
            return entries.find((entry) => entry.snapshot.url === tab.url)
              .snapshot;
          },
          { ...tab, url: origin + path },
        );
        assert.doesNotMatch(
          JSON.stringify(privateSnapshot),
          /PRIVATE_DRAFT_SENTINEL/,
        );
        assert.equal(privateSnapshot.images.length, 0);
        if (path === "/selection")
          assert.equal(privateSnapshot.text, "Public introduction.");
      }
      const manifest = JSON.parse(
        await readFile("extension/manifest.json", "utf8"),
      );
      assert.equal(manifest.action.default_popup, "capture.html?popup=1");
      assert.ok(manifest.commands._execute_action);
      assert.ok(!manifest.permissions.includes("cookies"));
      assert.deepEqual(manifest.host_permissions, ["http://127.0.0.1/*"]);
    } finally {
      await context?.close();
      await receiver.close();
      await new Promise<void>((r) => website.close(() => r()));
    }
  },
);
