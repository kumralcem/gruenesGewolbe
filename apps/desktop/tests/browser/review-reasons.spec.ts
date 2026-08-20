import { expect, test } from "@playwright/test";
import type { ItemDetails } from "../../src/contracts";

test("opens Review Queue reasons and resolves each concern independently", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    let item: ItemDetails = {
      id: "item-mystery",
      home_subvault: "Paintings",
      item_folder: "/vault/subvaults/Paintings/items/mystery",
      title: "Mystery",
      creator: "Unknown Creator",
      year: "Unknown Year",
      primary_file: "/vault/subvaults/Paintings/items/mystery/files/mystery.jpg",
      review_status: "needs-review",
      review_reasons: [
        {
          id: "unknown-creator",
          kind: "unknown-metadata",
          target_field: "creator",
          message: "Creator is unknown",
          evidence: "No creator metadata was inferred",
        },
        {
          id: "unknown-year",
          kind: "unknown-metadata",
          target_field: "year",
          message: "Year is unknown",
          evidence: "No year metadata was inferred",
        },
      ],
      tags: [],
      collections: [],
      item_links: [],
      saving_reason: null,
      source_link: null,
      summary: null,
      source_copy: null,
      record_revision: "revision-1",
      folder_rename_proposal: null,
    };
    Object.assign(window, { __testCalls: calls });
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
      workbenchSnapshot: async (_sort, selectedItemId) => ({
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
            thumbnail_file: "/derived/mystery.png",
            thumbnail_is_placeholder: false,
            review_status: item.review_status,
          },
        ],
        idea_sources: [],
        review_queue:
          item.review_reasons.length === 0
            ? []
            : [
                {
                  id: item.id,
                  home_subvault: item.home_subvault,
                  item_type: "artwork",
                  title: item.title,
                  review_status: item.review_status,
                  saving_reason: null,
                  review_reasons: item.review_reasons,
                },
              ],
        search_results: [],
        selected_item: selectedItemId ? { ...item } : null,
      }),
      resolveReviewReason: async ({ reason_id: reasonId, action, correction }) => {
        calls.push(`resolve:${reasonId}:${action}:${correction ?? ""}`);
        if (reasonId === "unknown-creator" && action === "correct") {
          item = { ...item, creator: correction ?? item.creator };
        }
        item = {
          ...item,
          review_reasons: item.review_reasons.filter((reason) => reason.id !== reasonId),
          record_revision: `${item.record_revision}-resolved`,
        };
        item.review_status = item.review_reasons.length === 0 ? "reviewed" : "needs-review";
        return { ...item };
      },
      fileUrl: (path) => `https://asset.localhost/${path.split("/").at(-1)}`,
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Mystery Creator is unknown/ }).click();
  const creatorReason = page.locator('[data-review-reason="unknown-creator"]');
  await expect(creatorReason).toBeFocused();
  await creatorReason.getByLabel("Correct creator").fill("Jane Painter");
  await creatorReason.getByRole("button", { name: "Correct" }).click();
  await expect(page.getByText("Jane Painter").first()).toBeVisible();
  await expect(page.getByText("Year is unknown").first()).toBeVisible();
  await expect.poll(() => readCalls(page)).toContain(
    "resolve:unknown-creator:correct:Jane Painter",
  );

  const yearReason = page.locator('[data-review-reason="unknown-year"]');
  await yearReason.getByRole("button", { name: "Dismiss" }).click();
  await expect(page.getByText("No unresolved Review Reasons.")).toBeVisible();
  await expect(page.getByText("reviewed", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Review Queue")).toHaveCount(0);
});

async function readCalls(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(
    () => (window as typeof window & { __testCalls?: string[] }).__testCalls ?? [],
  );
}
