import { expect, test } from "@playwright/test";

const pixel =
  "data:image/svg+xml," +
  encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="80" height="60"><rect width="80" height="60" fill="#315f4d"/></svg>');

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
