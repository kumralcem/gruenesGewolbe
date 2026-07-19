import { expect, test } from "@playwright/test";

test("first launch offers only functional vault actions", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: null,
        known_vaults: [],
        notice: null,
      }),
      selectFolder: async () => null,
      createVault: async () => {
        throw new Error("not used");
      },
      openVault: async () => {
        throw new Error("not used");
      },
    };
  });

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Gruenes Gewoelbe" })).toBeVisible();
  await expect(page.getByTestId("active-vault")).toHaveText("No vault open");
  await expect(page.getByRole("main").getByRole("button", { name: "Create Vault" })).toBeVisible();
  await expect(page.getByRole("main").getByRole("button", { name: "Open Vault" })).toBeVisible();
  await expect(page.getByText(/capture|openai|search|import/i)).toHaveCount(0);
});

test("creates a format-v2 vault in the selected folder", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: null, known_vaults: [], notice: null }),
      selectFolder: async () => "/home/cem/Archive",
      createVault: async (root) => {
        calls.push(`create:${root}`);
        return { root };
      },
      openVault: async () => {
        throw new Error("not used");
      },
    };
  });

  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Create Vault" }).click();

  await expect(page.getByTestId("active-vault")).toHaveText("/home/cem/Archive");
  await expect(page.getByRole("heading", { name: "Archive" })).toBeVisible();
  await expect.poll(() => readCalls(page)).toEqual(["create:/home/cem/Archive"]);
});

test("shows a creation refusal without activating the folder", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({ active_vault: null, known_vaults: [], notice: null }),
      selectFolder: async () => "/home/cem/Documents",
      createVault: async (root) => {
        throw new Error(`vault folder is not empty: ${root}`);
      },
      openVault: async () => {
        throw new Error("not used");
      },
    };
  });

  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Create Vault" }).click();

  await expect(page.getByRole("alert")).toHaveText(
    "vault folder is not empty: /home/cem/Documents",
  );
  await expect(page.getByTestId("active-vault")).toHaveText("No vault open");
});

test("opens a known vault from the navigation", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    Object.assign(window, { __testCalls: calls });
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: null,
        known_vaults: [{ root: "/vaults/Archive" }],
        notice: null,
      }),
      selectFolder: async () => null,
      createVault: async () => {
        throw new Error("not used");
      },
      openVault: async (root) => {
        calls.push(`open:${root}`);
        return { root };
      },
    };
  });

  await page.goto("/");
  await page.locator('[data-known-vault="/vaults/Archive"]').click();

  await expect(page.getByTestId("active-vault")).toHaveText("/vaults/Archive");
  await expect.poll(() => readCalls(page)).toEqual(["open:/vaults/Archive"]);
});

test("renders a restored last-active vault on restart", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: { root: "/vaults/Restored" },
        known_vaults: [{ root: "/vaults/Restored" }],
        notice: null,
      }),
      selectFolder: async () => null,
      createVault: async () => {
        throw new Error("not used");
      },
      openVault: async () => {
        throw new Error("not used");
      },
    };
  });

  await page.goto("/");

  await expect(page.getByTestId("active-vault")).toHaveText("/vaults/Restored");
  await expect(page.getByRole("heading", { name: "Restored" })).toBeVisible();
  await expect(page.getByText("Active", { exact: true })).toBeVisible();
});

test("keeps a missing last-active vault visible and explains the fallback", async ({ page }) => {
  await page.addInitScript(() => {
    window.__GG_TEST_ADAPTER__ = {
      startup: async () => ({
        active_vault: null,
        known_vaults: [{ root: "/media/offline/Archive" }],
        notice: "last active vault is unavailable: /media/offline/Archive",
      }),
      selectFolder: async () => null,
      createVault: async () => {
        throw new Error("not used");
      },
      openVault: async () => {
        throw new Error("not used");
      },
    };
  });

  await page.goto("/");

  await expect(page.getByRole("alert")).toHaveText(
    "last active vault is unavailable: /media/offline/Archive",
  );
  await expect(page.locator('[data-known-vault="/media/offline/Archive"]')).toBeVisible();
  await expect(page.getByTestId("active-vault")).toHaveText("No vault open");
});

async function readCalls(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(
    () => (window as typeof window & { __testCalls?: string[] }).__testCalls ?? [],
  );
}
