import { expect, test } from "@playwright/test";

for (const choice of ["Not a Duplicate", "Keep Both", "Move This Item to Vault Trash"] as const) {
  test(`shows comparison evidence and resolves with ${choice}`, async ({ page }) => {
    await page.addInitScript(() => {
      const calls: string[] = [];
      let resolved = false;
      let trashed = false;
      const candidate = { id: "candidate-1", title: "Nocturne research", source_link: "https://example.com/nocturne", source_copy: null, review_status: "reviewed", saving_reason: null };
      const current = { id: "current-1", title: "Nocturne Study copy", creator: "Jane Painter", year: "1884", primary_file: "/current.jpg", thumbnail_file: "/current-thumb.jpg", thumbnail_is_placeholder: false, review_status: "needs-review" };
      const reason = { id: "duplicate-candidate-candidate-1", kind: "duplicate-candidate", target_field: null, message: "Possible overlap with another Saved Item", evidence: "candidate-1: source-link", candidate_item_id: "candidate-1" };
      Object.assign(window, { __testCalls: calls });
      window.__GG_TEST_ADAPTER__ = {
        startup: async () => ({ active_vault: { root: "/vault" }, known_vaults: [{ root: "/vault" }], repair_proposal: null, notice: null }),
        selectFolder: async () => null,
        createVault: async () => { throw new Error("unused"); }, openVault: async () => { throw new Error("unused"); },
        confirmVaultRepair: async () => { throw new Error("unused"); }, cancelVaultRepair: async () => {},
        workbenchSnapshot: async (_sort, selected) => ({
          active_vault: { root: "/vault" }, subvaults: ["Paintings"], collections: [],
          artwork_items: trashed ? [] : [current], idea_sources: [candidate],
          review_queue: resolved ? [] : [{ id: current.id, home_subvault: "Paintings", item_type: "artwork", title: current.title, review_status: "needs-review", saving_reason: null, review_reasons: [reason] }],
          search_results: [], vault_problems: [], trashed_items: trashed ? [{ id: current.id, title: current.title, home_subvault: "Paintings", item_folder: "/vault/trash/Paintings/current", collections: [], incoming_item_links: [] }] : [],
          selected_item: selected && !trashed ? { ...current, home_subvault: "Paintings", item_folder: "/vault/current", review_reasons: resolved ? [] : [reason], duplicate_candidates: [{ item_id: candidate.id, signal: "descriptive-metadata" }], tags: [], collections: [], item_links: [], saving_reason: null, source_link: null, summary: null, source_copy: null, record_revision: "revision-1", folder_rename_proposal: null, review_status: resolved ? "reviewed" : "needs-review" } : null,
        }),
        resolveDuplicateCandidate: async ({ action }) => { calls.push(action); resolved = true; trashed = action === "move-this-item-to-vault-trash"; return { status: trashed ? "moved-to-vault-trash" : "active" }; },
        fileUrl: (path) => path,
      };
    });
    await page.goto("/");
    await page.getByRole("button", { name: /Nocturne Study copy Possible overlap/ }).click();
    await expect(page.getByText("Matching evidence: candidate-1: source-link")).toBeVisible();
    await expect(page.getByText("Nocturne research").last()).toBeVisible();
    await expect(page.getByText("https://example.com/nocturne").last()).toBeVisible();
    await page.getByRole("button", { name: choice, exact: true }).click();
    await expect(page.getByLabel("Review Queue")).toHaveCount(0);
    if (choice === "Move This Item to Vault Trash") {
      const trash = page.getByRole("region", { name: "Vault Trash" });
      await trash.locator("summary").click();
      await expect(trash.getByText("Nocturne Study copy")).toBeVisible();
    }
    await expect.poll(() => page.evaluate(() => (window as typeof window & { __testCalls?: string[] }).__testCalls)).toHaveLength(1);
  });
}
