import { expect, test } from "@playwright/test";

const detail = {
  id: "painting-1", home_subvault: "Paintings", item_folder: "/vault/Paintings/one",
  title: "Blue Study", creator: "A. Painter", year: "1901", primary_file: "/vault/one.jpg",
  review_status: "reviewed", review_reasons: [], tags: [], collections: [], item_links: [],
  saving_reason: null, source_link: null, summary: null, source_copy: null,
  record_revision: "r1", folder_rename_proposal: null,
};

function snapshot(selectedItemId: string | null = null) {
  return {
    active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [],
    artwork_items: [{ id: "painting-1", title: "Blue Study", creator: "A. Painter", year: "1901", primary_file: "/vault/one.jpg", thumbnail_file: "/vault/one.jpg", thumbnail_is_placeholder: false, review_status: "reviewed" }],
    idea_sources: [], review_queue: [], search_results: [], vault_problems: [], trashed_items: [],
    selected_item: selectedItemId ? detail : null,
  };
}

test("opens Settings without a Vault and persists bounded enrichment controls", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: null, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }),
      openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }),
      confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      configureOpenAiProvider: async () => {}, openAiProviderStatus: async () => ({ configured: false, model: null }),
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByLabel("Maximum paintings per run")).toHaveValue("25");
  await page.getByLabel("Maximum paintings per run").fill("7");
  await page.getByLabel("Maximum paintings per run").dispatchEvent("change");
  await page.getByRole("button", { name: "Back to archive" }).click();
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByLabel("Maximum paintings per run")).toHaveValue("7");
});

test("keeps artwork details sticky and refreshes locally on focus", async ({ page }) => {
  await page.addInitScript(({ detail, base }) => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }), openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }), confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selected) => { calls.push(`snapshot:${selected ?? "none"}`); return { ...base, selected_item: selected ? detail : null }; },
      getItemDetails: async () => detail,
    };
  }, { detail, base: { ...snapshot(), artwork_items: Array.from({ length: 48 }, (_, index) => ({ ...snapshot().artwork_items[0]!, id: index === 0 ? "painting-1" : `painting-${index + 1}`, title: index === 0 ? "Blue Study" : `Study ${index + 1}` })) } });
  await page.goto("/");
  await page.getByRole("button", { name: "Blue Study" }).click();
  await expect(page.getByRole("complementary", { name: "Artwork details" })).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Artwork details" })).toHaveCSS("position", "sticky");
  const navigation = page.locator("[data-archive-view=paintings]");
  const before = await navigation.boundingBox();
  await page.locator(".workspace").evaluate(element => { element.scrollTop = 650; });
  const inspector = await page.getByRole("complementary", { name: "Artwork details" }).boundingBox();
  const workspace = await page.locator(".workspace").boundingBox();
  expect(inspector!.y).toBeGreaterThanOrEqual(workspace!.y);
  expect(inspector!.y).toBeLessThan(workspace!.y + 100);
  expect((await navigation.boundingBox())!.y).toBe(before!.y);
  await page.screenshot({ path: "../../.scratch/artwork-organization/sidebar-scroll.png" });
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls.filter(call => call === "snapshot:painting-1").length)).toBeGreaterThan(0);
});

test("Paintings fallback preserves a pasted image through the artwork destination", async ({ page }) => {
  await page.addInitScript(({ base }) => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }), openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }), confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => base, captureSourceLink: async () => ({ status: "needs_manual_fallback" as const, source_link: "https://example.test/image", title: "Study", saving_reason: null, reason: "Image extraction failed." }),
      captureArtworkFallback: async request => { calls.push(`artwork:${request.copiedImage?.fileName}:${request.copiedText}`); return { id: "painting-2", home_subvault: "Paintings", item_folder: "/vault/Paintings/two" }; },
    };
  }, { base: snapshot() });
  await page.goto("/");
  await page.getByRole("button", { name: "Capture Link" }).click();
  await page.getByLabel("Source Link").fill("https://example.test/image");
  await page.getByRole("button", { name: "Capture", exact: true }).click();
  await page.locator('[name="copied_text"]').dispatchEvent("paste", { bubbles: true });
  await page.evaluate(() => {
    const field = document.querySelector<HTMLTextAreaElement>('[name="copied_text"]');
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array([1, 2, 3])], "study.png", { type: "image/png" }));
    field?.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, clipboardData: transfer }));
  });
  await page.getByRole("button", { name: "Save Artwork" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls)).toContain("artwork:study.png:null");
});

test("starts bounded enrichment and sends cancellation separately from focus refresh", async ({ page }) => {
  await page.addInitScript(({ base }) => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }), openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }), confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => { calls.push("local-refresh"); return base; },
      startArtworkEnrichment: async (_options, onProgress) => { calls.push("enrichment-start"); await onProgress({ runId: "run-1", processed: 0, total: 1, enriched: 0, failed: 0, skipped: 0, status: "running", currentItemTitle: "Blue Study" }); return await new Promise<never>(() => {}); },
      cancelArtworkEnrichment: async runId => { calls.push(`enrichment-cancel:${runId}`); },
    };
  }, { base: snapshot() });
  await page.goto("/");
  await page.getByRole("button", { name: "Refresh Item Records" }).click();
  await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls)).toContain("enrichment-cancel:run-1");
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls)).toContain("local-refresh");
});

test("recovers a paused enrichment checkpoint and resumes to completed", async ({ page }) => {
  await page.addInitScript(({ base }) => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }), openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }), confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => { calls.push("snapshot"); return base; },
      artworkEnrichmentStatus: async () => ({ runId: "checkpoint-1", processed: 1, total: 2, enriched: 1, failed: 0, skipped: 0, status: "paused" as const, currentItemTitle: "Second Study" }),
      resumeArtworkEnrichment: async (runId, onProgress) => { calls.push(`resume:${runId}`); const progress = { runId, processed: 2, total: 2, enriched: 2, failed: 0, skipped: 0, status: "completed" as const, currentItemTitle: null }; await onProgress(progress); return progress; },
    };
  }, { base: snapshot() });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Resume" })).toBeVisible();
  await page.getByRole("button", { name: "Resume" }).click();
  await expect(page.getByText(/Artwork enrichment completed/)).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls)).toContain("resume:checkpoint-1");
  await expect.poll(() => page.evaluate(() => (window as unknown as { __testCalls: string[] }).__testCalls.filter(call => call === "snapshot").length)).toBeGreaterThan(1);
});

test("shows a rejected provider without leaving a batch running", async ({ page }) => {
  await page.addInitScript(({ base }) => {
    let rejected = false;
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async () => ({ root: "/vault" }),
      openVault: async () => ({ status: "opened" as const, vault: { root: "/vault" } }),
      confirmVaultRepair: async () => ({ root: "/vault" }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => base,
      startArtworkEnrichment: async () => { rejected = true; throw new Error("OpenAI provider rejected the request: model unavailable"); },
      artworkEnrichmentStatus: async () => rejected ? {
        runId: "rejected-run", processed: 1, total: 2, enriched: 0, failed: 1, skipped: 0,
        status: "paused" as const, currentItemTitle: null,
        failures: [{ itemId: "painting-1", title: "Blue Study", reason: "Model unavailable" }],
      } : null,
    };
  }, { base: snapshot() });
  await page.goto("/");
  await page.getByRole("button", { name: "Refresh Item Records" }).click();
  await expect(page.getByText("OpenAI provider rejected the request: model unavailable")).toBeVisible();
  await expect(page.getByText("Artwork enrichment paused")).toBeVisible();
  await expect(page.getByRole("button", { name: "Cancel", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Refresh Item Records" })).toBeEnabled();
  await page.getByText("See 1 failures").click();
  await expect(page.getByText("Blue Study: Model unavailable")).toBeVisible();
});
