import { expect, test } from "@playwright/test";

const pixel =
  "data:image/svg+xml," +
  encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="80" height="60"><rect width="80" height="60" fill="#315f4d"/></svg>');

test("keeps delayed Item Details responsive and scoped to the latest selection", async ({ page }) => {
  await page.addInitScript((imageUrl) => {
    const items = [1, 2, 3].map((number) => ({
      id: `item-${number}`, title: `Artwork ${number}`, creator: "Artist", year: "2024",
      primary_file: `/vault/item-${number}.jpg`, thumbnail_file: `/derived/item-${number}.png`,
      thumbnail_is_placeholder: false, review_status: "reviewed",
    }));
    const secondVaultItem = {
      id: "item-b", title: "Vault B Artwork", creator: "Artist", year: "2025",
      primary_file: "/vault-b/item-b.jpg", thumbnail_file: "/derived/item-b.png",
      thumbnail_is_placeholder: false, review_status: "reviewed",
    };
    let activeRoot = "/vault";
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault-b" }], repair_proposal: null, notice: null,
      }),
      selectFolder: async () => null,
      createVault: async (root) => ({ root }),
      openVault: async (root) => {
        activeRoot = root;
        return { status: "opened" as const, vault: { root } };
      },
      confirmVaultRepair: async (root) => ({ root }),
      cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId) => {
        const requestedRoot = activeRoot;
        if (selectedItemId === "item-1") await new Promise((resolve) => setTimeout(resolve, 600));
        if (selectedItemId === "item-2") await new Promise((resolve) => setTimeout(resolve, 50));
        if (selectedItemId === "item-3") {
          await new Promise((resolve) => setTimeout(resolve, 50));
          throw new Error("Item Record could not be read");
        }
        const selected = items.find((item) => item.id === selectedItemId);
        if (requestedRoot === "/vault-b") {
          return {
            active_vault: { root: requestedRoot }, subvaults: ["Paintings"], collections: [],
            artwork_items: [secondVaultItem], idea_sources: [], review_queue: [], search_results: [],
            vault_problems: [], trashed_items: [], selected_item: null,
          };
        }
        return {
          active_vault: { root: requestedRoot }, subvaults: ["Paintings"], collections: [],
          artwork_items: items, idea_sources: [], review_queue: [], search_results: [],
          vault_problems: [], trashed_items: [],
          selected_item: selected ? {
            ...selected, home_subvault: "Paintings", item_folder: `/vault/${selected.id}`,
            review_reasons: [], tags: [], collections: [], item_links: [], saving_reason: null,
            source_link: null, summary: null, source_copy: null, record_revision: "r1",
            folder_rename_proposal: null,
          } : null,
        };
      },
      fileUrl: () => imageUrl,
    };
  }, pixel);
  await page.goto("/");

  await page.locator('[data-artwork-id="item-1"]').click();
  await expect(page.getByRole("img", { name: "Preview of Artwork 1" })).toBeVisible({ timeout: 150 });
  await page.getByRole("button", { name: "Close Item Details" }).click();
  await expect(page.getByRole("complementary", { name: "Artwork details" })).toHaveCount(0);
  await page.waitForTimeout(700);
  await expect(page.getByRole("complementary", { name: "Artwork details" })).toHaveCount(0);

  await page.locator('[data-artwork-id="item-1"]').click();
  await page.locator('[data-artwork-id="item-2"]').click();
  await expect(page.getByRole("img", { name: "Preview of Artwork 2" })).toBeVisible({ timeout: 150 });
  await expect(page.locator('[data-item-record-form] input[name="title"]')).toHaveValue("Artwork 2");
  await page.waitForTimeout(600);
  await expect(page.getByRole("heading", { name: "Artwork 2" })).toBeVisible();

  const shell = await page.locator(".app-shell").elementHandle();
  await page.locator('[data-artwork-id="item-3"]').click();
  await expect(page.getByRole("img", { name: "Preview of Artwork 3" })).toBeVisible({ timeout: 150 });
  await expect(page.getByText("Item Record could not be read")).toBeVisible();
  expect(await shell!.evaluate((element) => element.isConnected)).toBe(true);
  await expect(page.getByRole("heading", { name: "Artwork 3" })).toBeVisible();

  await page.locator('[data-artwork-id="item-1"]').click();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.waitForTimeout(700);
  await expect(page.getByRole("heading", { name: "Artwork 1" })).toHaveCount(0);
  await expect(page.locator('[data-item-record-form] input[name="title"]')).toHaveValue("Artwork 2");

  await page.locator('[data-artwork-id="item-1"]').click();
  await page.locator('[data-known-vault="/vault-b"]').click();
  await expect(page.getByTestId("active-vault")).toHaveText("/vault-b");
  await expect(page.getByRole("button", { name: /Vault B Artwork/ })).toBeVisible();
  await page.waitForTimeout(700);
  await expect(page.getByRole("heading", { name: "Artwork 1" })).toHaveCount(0);
});

test("uses lightweight Item Details for artwork selection and ignores stale responses", async ({ page }) => {
  await page.addInitScript((imageUrl) => {
    const items = [1, 2].map((number) => ({
      id: `item-${number}`, title: `Artwork ${number}`, creator: "Artist", year: "2024",
      primary_file: `/vault/item-${number}.jpg`, thumbnail_file: `/derived/item-${number}.png`,
      thumbnail_is_placeholder: false, review_status: "reviewed",
    }));
    const details = (item: (typeof items)[number]) => ({
      ...item, home_subvault: "Paintings", item_folder: `/vault/${item.id}`,
      review_reasons: [], tags: [], collections: [], item_links: [], saving_reason: null,
      source_link: null, summary: null, source_copy: null, record_revision: "r1",
      folder_rename_proposal: null,
    });
    let snapshotCalls = 0;
    (window as any).__selectionSnapshotCalls = () => snapshotCalls;
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null,
      createVault: async (root) => ({ root }),
      openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
      confirmVaultRepair: async (root) => ({ root }),
      cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => {
        snapshotCalls += 1;
        return {
          active_vault: { root: "/vault" }, subvaults: ["Paintings"], collections: [], artwork_items: items,
          idea_sources: [], review_queue: [], search_results: [], vault_problems: [], trashed_items: [], selected_item: null,
        };
      },
      getItemDetails: async (id) => {
        await new Promise((resolve) => setTimeout(resolve, id === "item-1" ? 180 : 10));
        return details(items.find((item) => item.id === id)!);
      },
      fileUrl: () => imageUrl,
    };
  }, pixel);
  await page.goto("/");

  await page.locator('[data-artwork-id="item-1"]').click();
  await page.locator('[data-artwork-id="item-2"]').click();
  await expect(page.getByRole("heading", { name: "Artwork 2" })).toBeVisible();
  await page.waitForTimeout(220);
  await expect(page.getByRole("heading", { name: "Artwork 2" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__selectionSnapshotCalls())).toBe(1);
});

test("opens an Idea Source reader when selected from search results", async ({ page }) => {
  await page.addInitScript(() => {
    const idea = {
      id: "idea-1", title: "Useful source", home_subvault: "Idea Sources", source_link: "https://example.com/post",
      source_copy: "source-copies/cleaned-text.md", review_status: "reviewed", saving_reason: null,
    };
    const details = {
      ...idea, home_subvault: "Idea Sources", item_folder: "/vault/ideas/idea-1", creator: "Unknown Creator",
      year: "Unknown Year", primary_file: "", review_reasons: [], tags: [], collections: [], item_links: [],
      summary: null, record_revision: "r1", folder_rename_proposal: null,
    };
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null,
      createVault: async (root) => ({ root }),
      openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
      confirmVaultRepair: async (root) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId, query) => ({
        active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [], artwork_items: [],
        idea_sources: [idea], review_queue: [], search_results: query ? [idea] : [], vault_problems: [], trashed_items: [],
        selected_item: selectedItemId === idea.id ? details : null,
      }),
      readIdeaSource: async () => ({ id: idea.id, source_link: idea.source_link, cleaned_text: "Preserved source text.", summary: null }),
    };
  });
  await page.goto("/");
  await page.getByLabel("Search Active Vault").fill("Useful");
  await page.getByRole("button", { name: "Search" }).click();
  await page.getByRole("button", { name: "Useful source" }).click();
  await expect(page.getByRole("button", { name: /Idea Sources/ })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("complementary", { name: "Idea Source details" })).toContainText("Preserved source text.");
});

test("ignores a delayed Idea Source reader after switching to Paintings", async ({ page }) => {
  await page.addInitScript(() => {
    const idea = {
      id: "idea-1", title: "Delayed source", home_subvault: "Idea Sources",
      source_link: "https://example.com/post", source_copy: "source-copies/text.md",
      review_status: "reviewed", saving_reason: null,
    };
    const details = {
      ...idea, item_folder: "/vault/ideas/idea-1", creator: "Unknown Creator", year: "Unknown Year",
      primary_file: "", review_reasons: [], tags: [], collections: [], item_links: [], summary: null,
      record_revision: "r1", folder_rename_proposal: null,
    };
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null,
      createVault: async (root) => ({ root }),
      openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
      confirmVaultRepair: async (root) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId) => ({
        active_vault: { root: "/vault" }, subvaults: ["Paintings", "Idea Sources"], collections: [],
        artwork_items: [], idea_sources: [idea], review_queue: [], search_results: [], vault_problems: [], trashed_items: [],
        selected_item: selectedItemId === idea.id ? details : null,
      }),
      readIdeaSource: async () => {
        await new Promise((resolve) => setTimeout(resolve, 180));
        return { id: idea.id, source_link: idea.source_link, cleaned_text: "Late source text.", summary: null };
      },
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: /Idea Sources/ }).click();
  await page.getByRole("button", { name: "Delayed source" }).click();
  await page.getByRole("button", { name: "Paintings" }).click();
  await page.waitForTimeout(220);
  await expect(page.getByRole("button", { name: "Paintings" })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("complementary", { name: "Idea Source details" })).toHaveCount(0);
});

test("keeps navigation in the viewport and opens dismissible details without jumping", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1200, height: 650 });
  await page.addInitScript((imageUrl) => {
    const items = Array.from({ length: 80 }, (_, index) => ({
      id: `item-${index}`,
      title: `Artwork ${index}`,
      creator: "Artist",
      year: "2024",
      primary_file: `/vault/item-${index}.jpg`,
      thumbnail_file: `/derived/item-${index}.png`,
      thumbnail_is_placeholder: false,
      review_status: "reviewed",
    }));
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: { root: "/vault" },
        known_vaults: [{ root: "/vault" }],
        repair_proposal: null,
        notice: null,
      }),
      selectFolder: async () => null,
      createVault: async (root) => ({ root }),
      openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
      confirmVaultRepair: async (root) => ({ root }),
      cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId) => ({
        active_vault: { root: "/vault" },
        subvaults: ["Paintings", "Idea Sources"],
        collections: [], artwork_items: items, idea_sources: [], review_queue: [], search_results: [],
        vault_problems: [], trashed_items: [],
        selected_item: selectedItemId
          ? {
              ...items.find((item) => item.id === selectedItemId)!,
              home_subvault: "Paintings", item_folder: `/vault/${selectedItemId}`,
              review_reasons: [], tags: [], collections: [], item_links: [], saving_reason: null,
              source_link: null, summary: null, source_copy: null, record_revision: "r1",
              folder_rename_proposal: null,
            }
          : null,
      }),
      fileUrl: () => imageUrl,
    };
  }, pixel);

  await page.goto("/");
  await expect(page.getByRole("button", { name: "Open Vault" })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBe(650);

  const workspace = page.locator(".workspace");
  const shell = await page.locator(".app-shell").elementHandle();
  expect(shell).not.toBeNull();
  await page.locator('[data-artwork-id="item-30"]').scrollIntoViewIfNeeded();
  const scrollBefore = await workspace.evaluate((element) => element.scrollTop);
  expect(scrollBefore).toBeGreaterThan(0);
  await page.locator('[data-artwork-id="item-30"]').click();

  await expect(page.getByRole("heading", { name: "Artwork 30" })).toBeVisible();
  expect(await shell!.evaluate((element) => element.isConnected)).toBe(true);
  await expect(page.getByRole("button", { name: "Close Item Details" })).toBeVisible();
  const scrollAfter = await workspace.evaluate((element) => element.scrollTop);
  expect(scrollAfter).toBeGreaterThan(0);
  expect(Math.abs(scrollAfter - scrollBefore)).toBeLessThan(650);

  await page.getByRole("button", { name: "Close Item Details" }).click();
  await expect(page.getByRole("complementary", { name: "Artwork details" })).toHaveCount(0);
});

test("places Vault Trash after the Saved Item gallery", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async (root) => ({ root }),
      openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
      confirmVaultRepair: async (root) => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings"], collections: [], artwork_items: [], idea_sources: [], review_queue: [], search_results: [], selected_item: null, vault_problems: [], trashed_items: [] }),
    };
  });
  await page.goto("/");

  const galleryPrecedesTrash = await page.evaluate(() => {
    const gallery = document.querySelector(".artwork-content");
    const trash = document.querySelector(".vault-trash");
    return Boolean(gallery && trash && gallery.compareDocumentPosition(trash) & Node.DOCUMENT_POSITION_FOLLOWING);
  });
  expect(galleryPrecedesTrash).toBe(true);
});
