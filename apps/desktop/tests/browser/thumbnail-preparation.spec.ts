import { expect, test } from "@playwright/test";

test("shows the gallery before preparing missing Thumbnail Previews in background batches", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let prepared = false;
    let finishBatch: (() => void) | null = null;
    let finishSnapshot: (() => void) | null = null;
    let snapshotCalls = 0;
    const snapshot = (selectedItemId: string | null = null) => ({
      active_vault: { root: "/vault" },
      subvaults: ["Paintings"],
      collections: [],
      artwork_items: [{
        id: "large",
        title: "Large Artwork",
        creator: "Artist",
        year: "2024",
        primary_file: "/vault/large.webp",
        thumbnail_file: prepared ? "/derived/large.png" : "/derived/large.pending.png",
        thumbnail_is_placeholder: !prepared,
        review_status: "reviewed",
      }],
      idea_sources: [], review_queue: [], search_results: [],
      selected_item: selectedItemId ? {
        id: "large", title: "Large Artwork", creator: "Artist", year: "2024",
        primary_file: "/vault/large.webp", home_subvault: "Paintings",
        item_folder: "/vault/large", review_status: "reviewed", review_reasons: [],
        tags: [], collections: [], item_links: [], saving_reason: null, source_link: null,
        summary: null, source_copy: null, record_revision: "r1", folder_rename_proposal: null,
      } : null,
      vault_problems: [], trashed_items: [],
    });
    Object.assign(window, {
      __finishThumbnailBatch: () => finishBatch?.(),
      __finishThumbnailSnapshot: () => finishSnapshot?.(),
      __thumbnailSnapshotCalls: () => snapshotCalls,
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }),
      openVault: async root => ({ status: "opened", vault: { root } }),
      confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId) => {
        snapshotCalls += 1;
        if (snapshotCalls === 2) {
          await new Promise<void>((resolve) => { finishSnapshot = resolve; });
        }
        return snapshot(selectedItemId);
      },
      prepareThumbnailPreviews: async () => {
        await new Promise<void>((resolve) => { finishBatch = resolve; });
        prepared = true;
        return { generated: 1, remaining: 0 };
      },
      fileUrl: path => path,
    };
  });

  await page.goto("/");
  const image = page.locator('[data-artwork-id="large"] img');
  await expect(image).toHaveAttribute("src", "/derived/large.pending.png");
  await page.evaluate(() => (window as any).__finishThumbnailBatch());
  await expect.poll(() => page.evaluate(() => (window as any).__thumbnailSnapshotCalls())).toBe(2);
  await page.locator('[data-artwork-id="large"]').click();
  await expect(page.locator("[data-item-record-form]")).toBeVisible();
  const form = page.locator("[data-item-record-form]");
  const title = form.locator('[name="title"]');
  await title.fill("Unsaved title draft");
  const originalForm = await form.elementHandle();
  await page.evaluate(() => (window as any).__finishThumbnailSnapshot());
  await expect(image).toHaveAttribute("src", "/derived/large.png");
  await expect(title).toHaveValue("Unsaved title draft");
  expect(await originalForm?.evaluate((element) => element.isConnected)).toBe(true);
});

test("does not rebuild the full workbench after every background Thumbnail Preview", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let remaining = 12;
    let snapshotCalls = 0;
    let preparationCalls = 0;
    const preparationLimits: number[] = [];
    const snapshot = () => ({
      active_vault: { root: "/vault" },
      subvaults: ["Paintings"], collections: [],
      artwork_items: Array.from({ length: 12 }, (_, index) => ({
        id: `item-${index}`,
        title: `Artwork ${index}`,
        creator: "Artist",
        year: "2024",
        primary_file: `/vault/item-${index}.jpg`,
        thumbnail_file: index < 12 - remaining
          ? `/derived/item-${index}.png`
          : "/derived/pending.png",
        thumbnail_is_placeholder: index >= 12 - remaining,
        review_status: "reviewed",
      })),
      idea_sources: [], review_queue: [], search_results: [], selected_item: null,
      vault_problems: [], trashed_items: [],
    });
    Object.assign(window, {
      __thumbnailMetrics: () => ({ snapshotCalls, preparationCalls, preparationLimits }),
      __GG_TEST_ADAPTER__: {
        startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
        selectFolder: async () => null, createVault: async (root: string) => ({ root }),
        openVault: async (root: string) => ({ status: "opened" as const, vault: { root } }),
        confirmVaultRepair: async (root: string) => ({ root }), cancelVaultRepair: async () => {},
        workbenchSnapshot: async () => {
          snapshotCalls += 1;
          return snapshot();
        },
        prepareThumbnailPreviews: async (limit: number) => {
          preparationCalls += 1;
          preparationLimits.push(limit);
          remaining -= 1;
          return { generated: 1, remaining };
        },
        fileUrl: (path: string) => path,
      },
    });
  });

  await page.goto("/");
  await expect.poll(async () => page.evaluate(() => (window as any).__thumbnailMetrics().preparationCalls)).toBe(12);
  const snapshotCalls = await page.evaluate(() => (window as any).__thumbnailMetrics().snapshotCalls);
  const preparationLimits = await page.evaluate(() => (window as any).__thumbnailMetrics().preparationLimits);

  expect(snapshotCalls).toBeLessThanOrEqual(4);
  expect(preparationLimits).toEqual(Array(12).fill(1));
});
