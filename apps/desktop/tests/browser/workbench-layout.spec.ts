import { expect, test } from "@playwright/test";

const pixel =
  "data:image/svg+xml," +
  encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="80" height="60"><rect width="80" height="60" fill="#315f4d"/><circle cx="40" cy="30" r="18" fill="#d8b66a"/></svg>');

for (const viewport of [
  { name: "desktop", width: 1440, height: 1000 },
  { name: "constrained", width: 720, height: 900 },
]) {
  test(`dense workbench remains coherent at the ${viewport.name} viewport`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.addInitScript((imageUrl) => {
      const item = {
        id: "item-layout",
        title: "Nocturne with an intentionally descriptive title",
        creator: "Jane Painter",
        year: "1884",
        primary_file: "/vault/items/nocturne/files/nocturne.png",
        thumbnail_file: "/vault/.gruenesgewolbe/thumbnails/item-layout.png",
        thumbnail_is_placeholder: false,
        review_status: "needs-review",
      };
      window.__GG_TEST_ADAPTER__ = {
        startup: async () => ({
          active_vault: { root: "/home/archive/Personal Archive" },
          known_vaults: [{ root: "/home/archive/Personal Archive" }],
          repair_proposal: null,
          notice: null,
        }),
        selectFolder: async () => null,
        createVault: async (root) => ({ root }),
        openVault: async (root) => ({ status: "opened" as const, vault: { root } }),
        confirmVaultRepair: async (root) => ({ root }),
        cancelVaultRepair: async () => {},
        workbenchSnapshot: async () => ({
          active_vault: { root: "/home/archive/Personal Archive" },
          subvaults: ["Paintings", "Idea Sources"],
          collections: [{ id: "night", name: "Night References" }],
          artwork_items: [item],
          idea_sources: [],
          review_queue: [
            {
              id: item.id,
              home_subvault: "Paintings",
              item_type: "artwork",
              title: item.title,
              review_status: "needs-review",
              saving_reason: "Palette and composition reference",
              review_reasons: [
                {
                  id: "thumbnail-preview-unavailable",
                  kind: "thumbnail-preview-unavailable",
                  target_field: null,
                  message: "Thumbnail Preview could not be decoded",
                  evidence: "The Preserved File remains available.",
                },
              ],
            },
          ],
          search_results: [{ id: item.id, home_subvault: "Paintings", title: item.title }],
          selected_item: null,
          vault_problems: [
            {
              path: "/home/archive/Personal Archive/subvaults/Paintings/items/broken/record.md",
              error: "Item Record YAML parse error: invalid sequence",
            },
          ],
          trashed_items: [
            {
              id: "item-trashed",
              title: "Garden Study",
              home_subvault: "Paintings",
              item_folder: "/home/archive/Personal Archive/trash/Paintings/Garden Study",
              collections: ["Night References"],
              incoming_item_links: [{ source_item_id: item.id, label: "Companion study" }],
            },
          ],
        }),
        fileUrl: () => imageUrl,
        captureSourceLink: async () => ({
          status: "captured" as const,
          item: { id: item.id, home_subvault: "Paintings", item_folder: "/vault/items/nocturne" },
        }),
        captureManualFallback: async () => ({
          id: "idea-1",
          home_subvault: "Idea Sources",
          item_folder: "/vault/ideas/idea-1",
        }),
      };
    }, pixel);

    await page.goto("/");
    await expect(page.getByRole("img", { name: /Nocturne/ })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Vault Problems" })).toBeVisible();
    await expect(page.getByRole("region", { name: "Vault Trash" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh Item Records" })).toBeVisible();

    const hasHorizontalOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
    );
    expect(hasHorizontalOverflow).toBe(false);

    await expect(page).toHaveScreenshot(`workbench-${viewport.name}.png`, {
      animations: "disabled",
      fullPage: true,
    });
  });
}
