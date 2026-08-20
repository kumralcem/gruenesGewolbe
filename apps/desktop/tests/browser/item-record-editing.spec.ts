import { expect, test } from "@playwright/test";
import type { ItemDetails } from "../../src/contracts";

test("edits an Item Record, resolves conflicts, refreshes, and confirms a folder rename", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    let conflictNextSave = false;
    let item: ItemDetails = {
      id: "item-draft",
      home_subvault: "Paintings",
      item_folder: "/vault/subvaults/Paintings/items/draft",
      title: "draft",
      creator: "Unknown Creator",
      year: "Unknown Year",
      primary_file: "/vault/subvaults/Paintings/items/draft/files/draft.jpg",
      review_status: "needs-review",
      review_reasons: [],
      tags: [] as string[],
      collections: ["Night References"],
      item_links: [
        {
          link_type: "url",
          target: "https://example.com/night-notes",
          label: "Night palette notes",
        },
      ],
      saving_reason: "Needs cleanup",
      source_link: null,
      summary: null,
      source_copy: null,
      record_revision: "revision-1",
      folder_rename_proposal: null,
    };
    Object.assign(window, {
      __testCalls: calls,
      __conflictNextSave: () => {
        conflictNextSave = true;
      },
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
      workbenchSnapshot: async (_sort, selectedItemId) => {
        calls.push(`refresh:${selectedItemId ?? "none"}`);
        return {
          active_vault: { root: "/vault" },
          subvaults: ["Paintings"],
          collections: [],
          artwork_items: [
            {
              id: item.id,
              title: item.title,
              creator: item.creator,
              year: item.year,
              primary_file: item.primary_file,
              thumbnail_file: "/derived/draft.png",
              thumbnail_is_placeholder: false,
              review_status: item.review_status,
            },
          ],
          idea_sources: [],
          review_queue: [],
          search_results: [],
          selected_item: selectedItemId ? { ...item } : null,
        };
      },
      saveItemRecord: async (edit) => {
        calls.push(`save:${edit.title}:${edit.overwrite_conflict}`);
        if (conflictNextSave && !edit.overwrite_conflict) {
          conflictNextSave = false;
          item = { ...item, title: "External File Title", record_revision: "revision-external" };
          return { status: "conflict", external_item: { ...item } };
        }
        item = {
          ...item,
          title: edit.title,
          creator: edit.creator,
          year: edit.year,
          saving_reason: edit.saving_reason || null,
          summary: edit.summary || null,
          tags: edit.tags,
          record_revision: `${item.record_revision}-saved`,
          folder_rename_proposal: {
            current_path: "/vault/subvaults/Paintings/items/draft",
            proposed_path:
              "/vault/subvaults/Paintings/items/Jane Painter - 1884 - Nocturne Study",
          },
        };
        return { status: "saved", item: { ...item } };
      },
      confirmItemFolderRename: async (id, proposal) => {
        calls.push(`rename:${id}:${proposal.proposed_path}`);
        item = {
          ...item,
          item_folder: "/vault/subvaults/Paintings/items/Jane Painter - 1884 - Nocturne Study",
          folder_rename_proposal: null,
        };
        return { ...item };
      },
      fileUrl: (path) => `https://asset.localhost/${path.split("/").at(-1)}`,
    };
  });

  await page.goto("/");
  await page.locator('[data-artwork-id="item-draft"]').click();
  await expect(page.getByText("Night palette notes")).toBeVisible();
  const editor = page.locator("[data-item-record-form]");
  await editor.getByLabel("Title").fill("");
  await page.getByRole("button", { name: "Save Item Record" }).click();
  await expect.poll(() => readCalls(page)).not.toContain("save::false");

  await editor.getByLabel("Title").fill("Nocturne Study");
  await editor.getByLabel("Creator").fill("Jane Painter");
  await editor.getByLabel("Year").fill("1884");
  await editor.getByLabel("Saving Reason").fill("Palette reference");
  await editor.getByLabel("Summary").fill("Blue-black night tones.");
  await editor.getByLabel("Tags").fill("night, atmosphere");
  await page.getByRole("button", { name: "Save Item Record" }).click();

  await expect(page.getByText("Folder rename available")).toBeVisible();
  await expect(page.getByText("/vault/subvaults/Paintings/items/draft", { exact: true })).toBeVisible();
  await expect(
    page.getByText("/vault/subvaults/Paintings/items/Jane Painter - 1884 - Nocturne Study"),
  ).toBeVisible();

  await page.evaluate(() =>
    (window as typeof window & { __conflictNextSave?: () => void }).__conflictNextSave?.(),
  );
  await editor.getByLabel("Title").fill("Editor Keeps This Draft");
  await page.getByRole("button", { name: "Save Item Record" }).click();
  await expect(page.getByText("Item Record Conflict")).toBeVisible();
  await expect(editor.getByLabel("Title")).toHaveValue("Editor Keeps This Draft");
  await expect(page.getByText("External File Title")).toBeVisible();

  await page.getByRole("button", { name: "Reload External Version" }).click();
  await expect(editor.getByLabel("Title")).toHaveValue("External File Title");

  await page.evaluate(() =>
    (window as typeof window & { __conflictNextSave?: () => void }).__conflictNextSave?.(),
  );
  await editor.getByLabel("Title").fill("Nocturne Study");
  await page.getByRole("button", { name: "Save Item Record" }).click();
  await page.getByRole("button", { name: "Overwrite After Review" }).click();
  await expect.poll(() => readCalls(page)).toContain("save:Nocturne Study:true");

  await page.getByRole("button", { name: "Refresh Item Records" }).click();
  const refreshesBeforeFocus = (await readCalls(page)).filter((call) => call.startsWith("refresh:")).length;
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect
    .poll(async () => (await readCalls(page)).filter((call) => call.startsWith("refresh:")).length)
    .toBeGreaterThan(refreshesBeforeFocus);

  await page.getByRole("button", { name: "Confirm Folder Rename" }).click();
  await expect.poll(() => readCalls(page)).toContain(
    "rename:item-draft:/vault/subvaults/Paintings/items/Jane Painter - 1884 - Nocturne Study",
  );
});

async function readCalls(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(
    () => (window as typeof window & { __testCalls?: string[] }).__testCalls ?? [],
  );
}
