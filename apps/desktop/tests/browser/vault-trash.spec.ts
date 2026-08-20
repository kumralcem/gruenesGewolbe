import { expect, test } from "@playwright/test";

test("moves an item to visible Vault Trash and restores it", async ({ page }) => {
  await page.addInitScript(() => {
    const active = { id: "item-1", title: "Blue", creator: "Artist", year: "2020", primary_file: "/vault/blue.png", thumbnail_file: "/vault/thumb.png", thumbnail_is_placeholder: false, review_status: "reviewed" };
    let trashed = false;
    const details = { ...active, home_subvault: "Paintings", item_folder: "/vault/subvaults/Paintings/items/Blue", review_reasons: [], tags: [], collections: ["Favorites"], item_links: [], saving_reason: null, source_link: null, summary: null, source_copy: null, record_revision: "r1", folder_rename_proposal: null };
    const snapshot = (selected: string | null) => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings"], collections: [], artwork_items: trashed ? [] : [active], idea_sources: [], review_queue: [], search_results: [], selected_item: !trashed && selected ? details : null, vault_problems: [], trashed_items: trashed ? [{ id: "item-1", title: "Blue", home_subvault: "Paintings", item_folder: "/vault/trash/Paintings/Blue", collections: ["Favorites"], incoming_item_links: [{ source_item_id: "item-2", label: "Inspired by" }] }] : [] });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }), openVault: async root => ({ status: "opened", vault: { root } }), confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selected) => snapshot(selected) as any,
      refreshWorkbenchSnapshot: async (_sort, selected) => snapshot(selected) as any,
      moveItemToTrash: async id => { trashed = true; return { id, home_subvault: "Paintings", item_folder: "/vault/trash/Paintings/Blue" }; },
      restoreTrashedItem: async id => { trashed = false; return { id, home_subvault: "Paintings", item_folder: "/vault/subvaults/Paintings/items/Blue" }; },
      fileUrl: path => path,
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: /Blue/ }).click();
  await page.getByRole("button", { name: "Move to Vault Trash" }).click();
  await expect(page.getByRole("region", { name: "Vault Trash" })).toContainText("Favorites");
  await expect(page.getByRole("region", { name: "Vault Trash" })).toContainText("Inspired by");
  await page.getByRole("button", { name: "Restore" }).click();
  await expect(page.getByRole("button", { name: /Blue/ })).toBeVisible();
});

test("permanent deletion shows impact and requires the stable item ID", async ({ page }) => {
  await page.addInitScript(() => {
    let deleted = false;
    const trashedItem = { id: "item-1", title: "Blue", home_subvault: "Paintings", item_folder: "/vault/trash/Paintings/Blue", collections: ["Favorites"], incoming_item_links: [{ source_item_id: "item-2", label: "Inspired by" }] };
    const snapshot = () => ({ active_vault: { root: "/vault" }, subvaults: ["Paintings"], collections: [], artwork_items: [], idea_sources: [], review_queue: [], search_results: [], selected_item: null, vault_problems: [], trashed_items: deleted ? [] : [trashedItem] });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
      selectFolder: async () => null, createVault: async root => ({ root }), openVault: async root => ({ status: "opened", vault: { root } }), confirmVaultRepair: async root => ({ root }), cancelVaultRepair: async () => {},
      workbenchSnapshot: async () => snapshot() as any, refreshWorkbenchSnapshot: async () => snapshot() as any,
      permanentlyDeleteTrashedItem: async (id, confirmedId) => { if (id !== confirmedId) throw new Error("exact item id required"); deleted = true; return { id, collections: ["Favorites"], incoming_item_links: [{ source_item_id: "item-2", label: "Inspired by" }] }; },
    };
  });
  await page.goto("/");
  const trash = page.getByRole("region", { name: "Vault Trash" });
  await expect(trash).toContainText("Favorites");
  await expect(trash).toContainText("Inspired by");
  await trash.getByRole("button", { name: "Permanently Delete" }).click();
  await expect(page.getByText("exact item id required")).toBeVisible();
  await trash.getByRole("textbox", { name: /Confirm permanent deletion/ }).fill("item-1");
  await trash.getByRole("button", { name: "Permanently Delete" }).click();
  await expect(trash).toContainText("Vault Trash is empty");
});
