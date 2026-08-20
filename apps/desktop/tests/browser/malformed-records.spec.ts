import { expect, test } from "@playwright/test";

test("keeps valid Saved Items usable around a malformed Item Record", async ({ page }) => {
  await page.addInitScript(() => {
    let corrected = false;
    const calls: string[] = [];
    const details = {
      id: "item-nocturne",
      home_subvault: "Paintings",
      item_folder: "/vault/subvaults/Paintings/items/nocturne",
      title: "Nocturne",
      creator: "Archive Artist",
      year: "2024",
      primary_file: "/vault/subvaults/Paintings/items/nocturne/files/nocturne.png",
      review_status: "reviewed",
      review_reasons: [],
      tags: [],
      collections: ["Night References"],
      item_links: [
        { link_type: "url", target: "https://example.com/notes", label: "Research notes" },
      ],
      saving_reason: null,
      source_link: null,
      summary: null,
      source_copy: null,
      record_revision: "revision-1",
      folder_rename_proposal: null,
    };
    const snapshot = (selectedItemId: string | null, searchQuery?: string | null) => ({
      active_vault: { root: "/vault" },
      subvaults: ["Paintings"],
      collections: [{ id: "night", name: "Night References" }],
      artwork_items: [
        {
          id: "item-nocturne",
          title: "Nocturne",
          creator: "Archive Artist",
          year: "2024",
          primary_file: details.primary_file,
          thumbnail_file: "/derived/nocturne.png",
          thumbnail_is_placeholder: false,
          review_status: "reviewed",
        },
        ...(corrected
          ? [
              {
                id: "item-garden",
                title: "Garden",
                creator: "Archive Artist",
                year: "2024",
                primary_file: "/vault/items/garden/files/garden.png",
                thumbnail_file: "/derived/garden.png",
                thumbnail_is_placeholder: false,
                review_status: "reviewed",
              },
            ]
          : []),
      ],
      idea_sources: [],
      review_queue: [],
      search_results:
        searchQuery === "nocturne"
          ? [{ id: "item-nocturne", home_subvault: "Paintings", title: "Nocturne" }]
          : [],
      selected_item: selectedItemId === "item-nocturne" ? details : null,
      vault_problems: corrected
        ? []
        : [
            {
              path: "/vault/subvaults/Paintings/items/garden/record.md",
              error: "Item Record YAML parse error: unexpected end of sequence",
            },
          ],
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: { root: "/vault" },
        known_vaults: [{ root: "/vault" }],
        repair_proposal: null,
        notice: null,
      }),
      selectFolder: async () => null,
      createVault: async () => {
        throw new Error("not used");
      },
      openVault: async () => {
        throw new Error("not used");
      },
      confirmVaultRepair: async () => {
        throw new Error("not used");
      },
      cancelVaultRepair: async () => {},
      workbenchSnapshot: async (_sort, selectedItemId, searchQuery) =>
        snapshot(selectedItemId, searchQuery),
      refreshWorkbenchSnapshot: async (_sort, selectedItemId, searchQuery) => {
        corrected = true;
        return snapshot(selectedItemId, searchQuery);
      },
      openActivityLog: async () => {
        calls.push("open-activity-log");
      },
      fileUrl: (path) => path,
    };
    (window as typeof window & { __calls?: string[] }).__calls = calls;
  });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Vault Problems" })).toBeVisible();
  await expect(page.getByText("record.md", { exact: true })).toBeVisible();
  await expect(page.getByText(/YAML parse error/)).toBeVisible();
  await expect(page.getByRole("img", { name: "Nocturne" })).toBeVisible();

  await page.getByLabel("Search Active Vault").fill("nocturne");
  await page.getByRole("button", { name: "Search" }).click();
  await page.getByRole("button", { name: "Nocturne", exact: true }).click();
  await expect(page.getByLabel("Artwork details").getByText("Night References")).toBeVisible();
  await expect(page.getByText("Research notes")).toBeVisible();

  await page.getByRole("button", { name: "Refresh Item Records" }).click();
  await expect(page.getByRole("heading", { name: "Vault Problems" })).toHaveCount(0);
  await expect(page.getByRole("img", { name: "Garden" })).toBeVisible();

  await page.getByRole("button", { name: "Open Activity Log" }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__calls)).toContain(
    "open-activity-log",
  );
});
