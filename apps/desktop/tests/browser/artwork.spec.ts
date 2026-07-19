import { expect, test } from "@playwright/test";

test("adds selected artwork files, sorts the gallery, and opens primary-file details", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    let loaded = false;
    Object.assign(window, { __testCalls: calls });
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
      cancelVaultRepair: async () => {
        throw new Error("not used");
      },
      selectArtworkFiles: async () => ["/imports/nocturne.jpg", "/imports/garden.png"],
      addArtworkFiles: async (sourceFiles, options) => {
        calls.push(
          `add:${sourceFiles.join("|")}:${options.metadata.creator}:${options.metadata.year}:${options.importExactDuplicates}`,
        );
        loaded = true;
        return {
          imported_count: 1,
          skipped_count: 1,
          duplicate_candidate_count: 0,
          exact_duplicate_count: 1,
          cancelled_count: 0,
          failed_count: 0,
          cancelled: false,
          imported_items: [
            { id: "item-garden", home_subvault: "Paintings", item_folder: "/items/garden" },
          ],
          skipped_entries: [
            {
              path: "/imports/nocturne.jpg",
              reason: "exact-file-duplicate",
              existing_item_id: "existing-nocturne",
            },
          ],
          duplicate_candidate_entries: [],
          cancelled_files: [],
          failed_entries: [],
          maintenance_errors: [],
          vault_problems: [],
        };
      },
      workbenchSnapshot: async (sort, selectedItemId) => {
        calls.push(`snapshot:${sort}:${selectedItemId ?? "none"}`);
        const artworkItems = loaded
          ? [
              {
                id: "item-nocturne",
                title: "Nocturne",
                creator: "Jane Painter",
                year: "1884",
                primary_file: "/items/nocturne/files/nocturne.jpg",
                thumbnail_file: "/derived/nocturne.png",
                thumbnail_is_placeholder: false,
                review_status: "reviewed",
              },
              {
                id: "item-garden",
                title: "Garden",
                creator: "Amy Artist",
                year: "2024",
                primary_file: "/items/garden/files/garden.png",
                thumbnail_file: "/derived/garden.png",
                thumbnail_is_placeholder: false,
                review_status: "reviewed",
              },
            ]
          : [];
        if (sort === "title") artworkItems.sort((left, right) => left.title.localeCompare(right.title));
        return {
          active_vault: { root: "/vaults/Archive" },
          subvaults: ["Paintings"],
          collections: [],
          artwork_items: artworkItems,
          idea_sources: [],
          review_queue: [],
          search_results: [],
          selected_item: selectedItemId
            ? {
                id: selectedItemId,
                home_subvault: "Paintings",
                item_folder: "/items/garden",
                title: "Garden",
                creator: "Amy Artist",
                year: "2024",
                primary_file: "/items/garden/files/garden.png",
                review_status: "reviewed",
                tags: [],
                collections: [],
                saving_reason: "Color reference",
                source_link: null,
                summary: null,
                source_copy: null,
              }
            : null,
        };
      },
      fileUrl: (path) => `https://asset.localhost/${path.split("/").at(-1)}`,
    };
  });

  await page.goto("/");
  await page.getByText("Optional metadata").click();
  await page.getByLabel("Creator").fill("Amy Artist");
  await page.getByLabel("Year").fill("2024");
  await page.getByRole("button", { name: "Add Artwork" }).click();

  await expect(page.getByText("1 exact duplicate")).toBeVisible();
  await expect(page.getByText("nocturne.jpg")).toBeVisible();

  await expect(page.getByRole("img", { name: "Nocturne" })).toHaveAttribute(
    "src",
    "https://asset.localhost/nocturne.png",
  );
  await expect(page.getByRole("img", { name: "Garden" })).toBeVisible();
  await page.getByLabel("Sort artwork").selectOption("title");
  await expect(page.locator("[data-artwork-id]").first()).toContainText("Garden");

  await page.locator('[data-artwork-id="item-garden"]').click();
  await expect(page.getByRole("heading", { name: "Garden" })).toBeVisible();
  await expect(page.getByLabel("Artwork details").getByText("Amy Artist · 2024")).toBeVisible();
  await expect(page.getByRole("img", { name: "Primary File for Garden" })).toHaveAttribute(
    "src",
    "https://asset.localhost/garden.png",
  );
  await expect.poll(() => readCalls(page)).toContain(
    "add:/imports/nocturne.jpg|/imports/garden.png:Amy Artist:2024:false",
  );
  await expect.poll(() => readCalls(page)).toContain("snapshot:title:item-garden");
});

async function readCalls(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(
    () => (window as typeof window & { __testCalls?: string[] }).__testCalls ?? [],
  );
}
