import { expect, test } from "@playwright/test";

test("offers Manual Fallback when a pasted source link cannot be extracted", async ({ page }) => {
  await page.addInitScript(() => {
    let captured = false;
    let manualImageBytes: number[] | null = null;
    (window as any).__manualImageBytes = () => manualImageBytes;
    const snapshot = () => ({
      active_vault: { root: "/vault" },
      subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [],
      idea_sources: captured ? [{
        id: "idea-1", title: "Saved post", source_link: "https://x.com/example/status/1",
        source_copy: "source-copies/cleaned-text.md", review_status: "needs-review",
        saving_reason: "Remember the composition notes",
      }] : [],
      review_queue: [], search_results: [], selected_item: null, vault_problems: [], trashed_items: [],
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }),
      openVault: async root => ({ status: "opened", vault: { root } }),
      confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      captureSourceLink: async (request) => ({
        status: "needs_manual_fallback", source_link: request.sourceLink,
        title: "", saving_reason: request.savingReason,
        reason: "X.com requires pasted content",
      }),
      captureManualFallback: async (request) => {
        manualImageBytes = request.copiedImage?.bytes ?? null;
        captured = true;
        return { id: "idea-1", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1" };
      },
      fileUrl: path => path,
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Capture Link" }).click();
  const capture = page.getByRole("region", { name: "Capture Link" });
  await capture.getByLabel("Source Link").fill("https://x.com/example/status/1");
  await capture.getByLabel(/Saving Reason/).fill("Remember the composition notes");
  await capture.getByRole("button", { name: "Try Capture" }).click();

  await expect(page.getByText("X.com requires pasted content")).toBeVisible();
  await expect(capture.getByLabel("Source Link")).toHaveValue("https://x.com/example/status/1");
  await capture.getByLabel("Title").fill("Saved post");
  await capture.getByLabel("Copied Text").evaluate((textarea) => {
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array([1, 2, 3])], "pasted.png", { type: "image/png" }));
    textarea.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, clipboardData: transfer }));
  });
  await expect(capture.getByText("pasted.png ready to preserve")).toBeVisible();
  await expect(capture.getByLabel("Title")).toHaveValue("Saved post");
  await capture.getByLabel("Copied Text").fill("The post text, without surrounding replies.");
  await capture.getByRole("button", { name: "Save Manual Fallback" }).click();

  await expect(page.getByRole("heading", { name: "Idea Sources" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Saved post" })).toBeVisible();
  await expect(page.getByText("https://x.com/example/status/1")).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__manualImageBytes())).toEqual([1, 2, 3]);
});

test("captures a supported Wikimedia source without opening Manual Fallback", async ({ page }) => {
  await page.addInitScript(() => {
    let captured = false;
    const snapshot = () => ({
      active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"],
      collections: [],
      artwork_items: captured ? [{
        id: "wave", title: "The Great Wave", creator: "Unknown Creator", year: "Unknown Year",
        primary_file: "/vault/Paintings/wave/files/wave.jpg", thumbnail_file: "/derived/wave.png",
        thumbnail_is_placeholder: false, review_status: "needs-review",
      }] : [],
      review_queue: [], search_results: [], selected_item: null,
      vault_problems: [], trashed_items: [],
      idea_sources: [],
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }),
      openVault: async root => ({ status: "opened", vault: { root } }),
      confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      captureSourceLink: async () => {
        captured = true;
        return { status: "captured", item: { id: "wave", home_subvault: "Paintings", item_folder: "/vault/Paintings/wave" } };
      },
      captureManualFallback: async () => { throw new Error("Manual Fallback should not open"); },
      fileUrl: path => path,
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Capture Link" }).click();
  const capture = page.getByRole("region", { name: "Capture Link" });
  await capture.getByLabel("Source Link").fill("https://commons.wikimedia.org/wiki/File:The_Great_Wave.jpg");
  await capture.getByLabel(/Saving Reason/).fill("Print reference");
  await capture.getByRole("button", { name: "Try Capture" }).click();

  await expect(capture).toHaveCount(0);
  await expect(page.getByRole("button", { name: /The Great Wave/ })).toBeVisible();
});
