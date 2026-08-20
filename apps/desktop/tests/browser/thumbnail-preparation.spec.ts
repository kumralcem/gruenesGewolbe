import { expect, test } from "@playwright/test";

test("shows the gallery before preparing missing Thumbnail Previews in background batches", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let prepared = false;
    let finishBatch: (() => void) | null = null;
    const snapshot = () => ({
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
      idea_sources: [], review_queue: [], search_results: [], selected_item: null,
      vault_problems: [], trashed_items: [],
    });
    Object.assign(window, { __finishThumbnailBatch: () => finishBatch?.() });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }),
      openVault: async root => ({ status: "opened", vault: { root } }),
      confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot(),
      prepareThumbnailPreviews: async () => {
        await new Promise<void>((resolve) => { finishBatch = resolve; });
        prepared = true;
        return { generated: 1, remaining: 0 };
      },
      fileUrl: path => path,
    };
  });

  await page.goto("/");
  const image = page.getByRole("img", { name: "Large Artwork" });
  await expect(image).toHaveAttribute("src", "/derived/large.pending.png");
  await page.evaluate(() => (window as any).__finishThumbnailBatch());
  await expect(image).toHaveAttribute("src", "/derived/large.png");
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
