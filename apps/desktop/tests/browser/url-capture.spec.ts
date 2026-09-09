import { expect, test } from "@playwright/test";

test("keeps an unsupported Idea Source draft until readable source text is supplied", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    let captured = false;
    const sourceText = "The post argues that references are most useful when the reason for saving them stays attached.";
    const details = {
      id: "idea-1", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1",
      title: "X post by example", creator: "Unknown Creator", year: "Unknown Year", primary_file: "",
      review_status: "needs-review", review_reasons: [], tags: [], collections: [], item_links: [],
      saving_reason: "Remember the composition notes", source_link: "https://x.com/example/status/1",
      summary: null, source_copy: "source-copies/cleaned-text.md", record_revision: "r1", folder_rename_proposal: null,
    };
    const snapshot = () => ({
      active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [],
      idea_sources: captured ? [{ id: "idea-1", title: details.title, source_link: details.source_link, source_copy: details.source_copy, review_status: details.review_status, saving_reason: details.saving_reason }] : [],
      review_queue: [], search_results: [], selected_item: captured ? details : null, vault_problems: [], trashed_items: [],
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async (root: string) => ({ root }),
      openVault: async (root: string) => ({ status: "opened", vault: { root } }),
      confirmVaultRepair: async (root: string) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      captureIdeaSource: async (request) => {
        if (!request.copiedText) return { status: "needs_manual_fallback", source_link: request.sourceLink, title: details.title, saving_reason: request.savingReason, reason: "This page could not be read automatically." };
        captured = request.copiedText === sourceText;
        return { status: "captured", item: { id: "idea-1", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1" }, summary_status: "unavailable", summary: null };
      },
      readIdeaSource: async () => ({ id: "idea-1", source_link: details.source_link, cleaned_text: sourceText, summary: null }),
      openAiProviderStatus: async () => ({ configured: false, model: null }),
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Idea Sources/ }).click();
  await page.getByRole("button", { name: "Add Idea Source" }).click();
  const capture = page.getByRole("region", { name: "Add Idea Source" });
  await capture.getByLabel("Source Link").fill("https://x.com/example/status/1");
  await capture.getByLabel(/Saving Reason/).fill("Remember the composition notes");
  await capture.getByRole("button", { name: "Save Idea Source" }).click();

  await expect(capture.getByText(/could not be read automatically/)).toBeVisible();
  await expect(capture.getByLabel("Source Link")).toHaveValue("https://x.com/example/status/1");
  await expect(page.getByText("No Idea Sources yet")).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.locator(".workspace").evaluate((element) => { element.scrollTop = 0; });
  await testInfo.attach("idea-source-manual-fallback", {
    body: await page.screenshot({ path: "../../.scratch/complete-idea-archive/evidence/idea-source-capture.png" }),
    contentType: "image/png",
  });
  await capture.getByLabel(/Source Text/).fill("The post argues that references are most useful when the reason for saving them stays attached.");
  await capture.getByRole("button", { name: "Save Source Text" }).click();

  const item = page.getByRole("complementary", { name: "Idea Source details" });
  await expect(item.getByText(/references are most useful/)).toBeVisible();
  await expect(item.getByText("Automatic summarization is unavailable until a provider is configured.")).toBeVisible();
  await expect(item.getByText("Source saved; summary unavailable. Check settings or retry.")).toBeVisible();
});

test("shows an automatically generated summary when a provider is configured", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    let captured = false;
    const details = { id: "idea-1", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1", title: "Useful post", creator: "Unknown Creator", year: "Unknown Year", primary_file: "", review_status: "reviewed", review_reasons: [], tags: [], collections: [], item_links: [], saving_reason: null, source_link: "https://example.com/post", summary: "A useful concise summary.", source_copy: "source-copies/cleaned-text.md", record_revision: "r1", folder_rename_proposal: null };
    const snapshot = () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [], idea_sources: captured ? [{ id: details.id, title: details.title, source_link: details.source_link, source_copy: details.source_copy, review_status: details.review_status, saving_reason: null }] : [], review_queue: [], search_results: [], selected_item: captured ? details : null, vault_problems: [], trashed_items: [] });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async (root: string) => ({ root }), openVault: async (root: string) => ({ status: "opened", vault: { root } }), confirmVaultRepair: async (root: string) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      captureIdeaSource: async (request) => { captured = request.copiedText === "A complete local source."; return { status: "captured", item: { id: "idea-1", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1" }, summary_status: "generated", summary: details.summary }; },
      readIdeaSource: async () => ({ id: "idea-1", source_link: details.source_link, cleaned_text: "A complete local source.", summary: details.summary }),
      openAiProviderStatus: async () => ({ configured: true, model: "summary-model" }),
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Idea Sources/ }).click();
  await page.getByRole("button", { name: "Add Idea Source" }).click();
  const capture = page.getByRole("region", { name: "Add Idea Source" });
  await capture.getByLabel("Source Link").fill("https://example.com/post");
  await capture.getByLabel(/Source Text/).fill("A complete local source.");
  await capture.getByRole("button", { name: "Save Idea Source" }).click();

  const item = page.getByRole("complementary", { name: "Idea Source details" });
  await expect(item.getByText("A complete local source.")).toBeVisible();
  await expect(item.getByRole("region", { name: "Summary" }).getByText("A useful concise summary.", { exact: true })).toBeVisible();
  await expect(item.getByText("Summary created and saved with the source.")).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.locator(".workspace").evaluate((element) => { element.scrollTop = 0; });
  await testInfo.attach("idea-source-reader", {
    body: await page.screenshot({ path: "../../.scratch/complete-idea-archive/evidence/idea-source-reader.png" }),
    contentType: "image/png",
  });
});

test("configures summarization outside the Vault without exposing the saved key", async ({ page }) => {
  await page.addInitScript(() => {
    let configured = false;
    let savedModel = "";
    (window as any).__providerConfiguration = () => ({ configured, savedModel });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async (root: string) => ({ root }), openVault: async (root: string) => ({ status: "opened", vault: { root } }), confirmVaultRepair: async (root: string) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [], idea_sources: [], review_queue: [], search_results: [], selected_item: null, vault_problems: [], trashed_items: [] }),
      captureIdeaSource: async () => { throw new Error("not called"); },
      configureOpenAiProvider: async ({ apiKey, model }) => { configured = apiKey === "secret-test-key"; savedModel = model; },
      openAiProviderStatus: async () => ({ configured, model: configured ? savedModel : null }),
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByText("Set up summaries").click();
  await page.getByLabel("OpenAI API Key").fill("secret-test-key");
  await page.getByLabel("Model").fill("gpt-4.1-mini");
  await page.getByRole("button", { name: "Save Summary Settings" }).click();

  await expect(page.getByText("Summaries: gpt-4.1-mini")).toBeVisible();
  await expect(page.getByText("secret-test-key")).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).__providerConfiguration())).toEqual({ configured: true, savedModel: "gpt-4.1-mini" });
});

test("persists Idea Source budget, skips summaries at Off, and uses the selected mode on retry", async ({ page }) => {
  const sourceText = "The preserved source remains readable when summaries are disabled.";
  await page.addInitScript((sourceText) => {
    let capturedMode = "";
    let retriedMode = "";
    let hasCapture = false;
    const details = { id: "idea-budget", home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-budget", title: "Budgeted source", creator: "Unknown Creator", year: "Unknown Year", primary_file: "", review_status: "reviewed", review_reasons: [], tags: [], collections: [], item_links: [], saving_reason: null, source_link: "https://example.com/budget", summary: null, source_copy: "source-copies/cleaned-text.md", record_revision: "r1", folder_rename_proposal: null };
    (window as any).__budgetModes = () => ({ capturedMode, retriedMode });
    const snapshot = () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [], idea_sources: hasCapture ? [{ id: details.id, title: details.title, source_link: details.source_link, source_copy: details.source_copy, review_status: details.review_status, saving_reason: null }] : [], review_queue: [], search_results: [], selected_item: hasCapture ? details : null, vault_problems: [], trashed_items: [] });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async (root: string) => ({ root }), openVault: async (root: string) => ({ status: "opened", vault: { root } }), confirmVaultRepair: async (root: string) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      captureIdeaSource: async (request) => { capturedMode = request.budgetMode; hasCapture = true; return { status: "captured", item: { id: details.id, home_subvault: "Idea Sources", item_folder: details.item_folder }, summary_status: request.budgetMode === "off" ? "skipped" : "generated", summary: null }; },
      readIdeaSource: async () => ({ id: details.id, source_link: details.source_link, cleaned_text: sourceText, summary: null }),
      summarizeIdeaSource: async (_id, mode) => { retriedMode = mode; return { status: "generated", summary: "Retried summary." }; },
      openAiProviderStatus: async () => ({ configured: true, model: "test-model" }),
    };
  }, sourceText);

  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Summary budget mode").selectOption("off");
  await page.reload();
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByLabel("Summary budget mode")).toHaveValue("off");
  await page.getByRole("button", { name: "Back to archive" }).click();
  await page.getByRole("button", { name: /Idea Sources/ }).click();
  await page.getByRole("button", { name: "Add Idea Source" }).click();
  const capture = page.getByRole("region", { name: "Add Idea Source" });
  await capture.getByLabel("Source Link").fill("https://example.com/budget");
  await capture.getByLabel(/Source Text/).fill(sourceText);
  await capture.getByRole("button", { name: "Save Idea Source" }).click();
  const item = page.getByRole("complementary", { name: "Idea Source details" });
  await expect(item.getByText(sourceText)).toBeVisible();
  await expect(item.getByText("Source saved without an automatic summary.")).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__budgetModes())).toEqual({ capturedMode: "off", retriedMode: "" });

  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Summary budget mode").selectOption("deep");
  await page.getByRole("button", { name: "Back to archive" }).click();
  await page.getByRole("button", { name: /Idea Sources/ }).click();
  await page.getByRole("button", { name: /Budgeted source/ }).click();
  await item.getByRole("button", { name: "Create Summary" }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__budgetModes().retriedMode)).toBe("deep");
});

test("never saves an empty fallback from the Paintings link capture", async ({ page }) => {
  await page.addInitScript(() => {
    let fallbackSaves = 0;
    (window as any).__fallbackSaves = () => fallbackSaves;
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }), openVault: async root => ({ status: "opened", vault: { root } }), confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [], idea_sources: [], review_queue: [], search_results: [], selected_item: null, vault_problems: [], trashed_items: [] }),
      captureSourceLink: async request => ({ status: "needs_manual_fallback", source_link: request.sourceLink, title: "Blocked page", saving_reason: request.savingReason, reason: "The page could not be extracted." }),
      captureArtworkFallback: async request => { fallbackSaves += 1; return { id: "painting-1", home_subvault: "Paintings", item_folder: `/vault/${request.title}` }; },
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Capture Link" }).click();
  const capture = page.getByRole("region", { name: "Capture Link" });
  await capture.getByLabel("Source Link").fill("https://example.com/blocked");
  await capture.getByRole("button", { name: "Capture", exact: true }).click();

  await expect(capture.getByText("The page could not be extracted.")).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__fallbackSaves())).toBe(0);
  await capture.getByRole("button", { name: "Save Artwork" }).click();
  await expect(page.getByRole("alert")).toContainText("Paste an image");
  await expect.poll(() => page.evaluate(() => (window as any).__fallbackSaves())).toBe(0);
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
  await capture.getByRole("button", { name: "Capture", exact: true }).click();

  await expect(capture).toHaveCount(0);
  await expect(page.getByRole("button", { name: /The Great Wave/ })).toBeVisible();
});
