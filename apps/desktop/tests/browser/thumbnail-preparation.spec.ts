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
