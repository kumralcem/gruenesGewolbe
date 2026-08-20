import { expect, test } from "@playwright/test";
import type { ImportRunSummary } from "../../src/contracts";

test("shows Import Run progress, supports cancellation, and renders the partial summary", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let finishImport: ((summary: ImportRunSummary) => void) | null = null;
    let imported = false;
    const snapshot = () => ({
      active_vault: { root: "/vaults/Archive" },
      subvaults: ["Paintings"],
      collections: [],
      artwork_items: imported
        ? [
            {
              id: "item-one",
              title: "One",
              creator: "Unknown Creator",
              year: "Unknown Year",
              primary_file: "/vaults/Archive/items/one/files/one.png",
              thumbnail_file: "/vaults/Archive/derived/one.png",
              thumbnail_is_placeholder: false,
              review_status: "needs-review",
            },
          ]
        : [],
      idea_sources: [],
      review_queue: [],
      search_results: [],
      vault_problems: [],
      selected_item: null,
    });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: { root: "/vaults/Archive" },
        known_vaults: [{ root: "/vaults/Archive" }],
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
      workbenchSnapshot: async () => snapshot(),
      selectImportFolder: async () => "/imports/paintings",
      runPaintingsImport: async (_sourceFolder, options, onProgress) => {
        if (!options.importExactDuplicates) throw new Error("override was not forwarded");
        await onProgress({ processed: 1, total: 3, current_file: "/imports/paintings/two.png" });
        return new Promise((resolve) => {
          finishImport = resolve;
        });
      },
      cancelPaintingsImport: async () => {
        imported = true;
        finishImport?.({
          imported_count: 1,
          skipped_count: 1,
          duplicate_candidate_count: 0,
          exact_duplicate_count: 0,
          cancelled_count: 1,
          failed_count: 0,
          cancelled: true,
          imported_items: [
            { id: "item-one", home_subvault: "Paintings", item_folder: "/items/one" },
          ],
          skipped_entries: [
            {
              path: "/imports/paintings/notes.txt",
              reason: "unsupported-file",
              existing_item_id: null,
            },
          ],
          duplicate_candidate_entries: [],
          cancelled_files: ["/imports/paintings/three.png"],
          failed_entries: [],
          maintenance_errors: [],
          vault_problems: [],
        });
      },
      fileUrl: (path) => path,
    };
  });

  await page.goto("/");
  await page.getByText("Optional metadata").click();
  await page.getByRole("checkbox", { name: "Import exact duplicates anyway" }).check();
  await page.getByRole("button", { name: "Import Folder" }).click();
  await expect(page.getByText("1 of 3 files processed")).toBeVisible();
  await expect(page.getByText("two.png")).toBeVisible();
  await page.getByRole("button", { name: "Cancel Import" }).click();

  await expect(page.getByRole("heading", { name: "Import cancelled" })).toBeVisible();
  await expect(page.getByText("1 imported")).toBeVisible();
  await expect(page.getByText("1 skipped")).toBeVisible();
  await expect(page.getByText("1 cancelled")).toBeVisible();
  await expect(page.getByText("notes.txt")).toBeVisible();
  await expect(page.getByText("three.png")).toBeVisible();
  await expect(page.getByRole("img", { name: "One" })).toBeVisible();
});
