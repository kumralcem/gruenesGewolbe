import {
  Archive,
  CircleAlert,
  FolderOpen,
  FolderPlus,
  Vault,
  createIcons,
} from "lucide";

import type {
  ActiveVault,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
} from "./contracts";

interface AppState extends DesktopStartup {
  busy: boolean;
  error: string | null;
}

export async function mountApp(root: HTMLElement, adapter: DesktopAdapter): Promise<void> {
  let state: AppState = {
    active_vault: null,
    known_vaults: [],
    repair_proposal: null,
    notice: null,
    busy: true,
    error: null,
  };

  const render = () => {
    root.innerHTML = pageTemplate(state);
    createIcons({
      icons: { Archive, CircleAlert, FolderOpen, FolderPlus, Vault },
      attrs: { "aria-hidden": "true", width: 18, height: 18 },
    });
    bindActions(root, adapter, state, async (nextState) => {
      state = nextState;
      render();
    });
  };

  render();
  try {
    state = { ...(await adapter.startup()), busy: false, error: null };
  } catch (error) {
    state = { ...state, busy: false, error: errorMessage(error) };
  }
  render();
}

function bindActions(
  root: HTMLElement,
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): void {
  root.querySelectorAll<HTMLButtonElement>("[data-vault-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const purpose = button.dataset.vaultAction as FolderPurpose;
      await chooseVault(purpose, adapter, state, update);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-known-vault]").forEach((button) => {
    button.addEventListener("click", async () => {
      const vaultRoot = button.dataset.knownVault;
      if (!vaultRoot) return;
      await openVault(vaultRoot, adapter, state, update);
    });
  });

  root.querySelector<HTMLButtonElement>("[data-repair-confirm]")?.addEventListener("click", async () => {
    const proposal = state.repair_proposal;
    if (!proposal) return;
    await activateVault(() => adapter.confirmVaultRepair(proposal.root), state, update);
  });

  root.querySelector<HTMLButtonElement>("[data-repair-cancel]")?.addEventListener("click", async () => {
    const proposal = state.repair_proposal;
    if (!proposal) return;
    await update({ ...state, busy: true, error: null });
    try {
      await adapter.cancelVaultRepair(proposal.root);
      await update({ ...state, repair_proposal: null, busy: false, error: null });
    } catch (error) {
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });
}

async function chooseVault(
  purpose: FolderPurpose,
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  let selected: string | null;
  try {
    selected = await adapter.selectFolder(purpose);
  } catch (error) {
    await update({ ...state, busy: false, error: errorMessage(error) });
    return;
  }
  if (!selected) return;

  if (purpose === "create") {
    await activateVault(() => adapter.createVault(selected), state, update);
  } else {
    await openVault(selected, adapter, state, update);
  }
}

async function openVault(
  root: string,
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  await update({ ...state, busy: true, error: null });
  try {
    const result = await adapter.openVault(root);
    if (result.status === "repair_required") {
      await update({
        ...state,
        repair_proposal: result.proposal,
        busy: false,
        error: null,
      });
      return;
    }
    await activateOpenedVault(result.vault, state, update);
  } catch (error) {
    await update({ ...state, busy: false, error: errorMessage(error) });
  }
}

async function activateVault(
  operation: () => Promise<ActiveVault>,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  await update({ ...state, busy: true, error: null });
  try {
    const activeVault = await operation();
    await activateOpenedVault(activeVault, state, update);
  } catch (error) {
    await update({ ...state, busy: false, error: errorMessage(error) });
  }
}

async function activateOpenedVault(
  activeVault: ActiveVault,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  const knownVaults = state.known_vaults.some((vault) => vault.root === activeVault.root)
    ? state.known_vaults
    : [...state.known_vaults, activeVault];
  await update({
    ...state,
    active_vault: activeVault,
    known_vaults: knownVaults,
    repair_proposal: null,
    notice: null,
    busy: false,
    error: null,
  });
}

function pageTemplate(state: AppState): string {
  return `
    <div class="app-shell" aria-busy="${state.busy}">
      <header class="app-bar">
        <div class="brand-lockup">
          <span class="brand-mark"><i data-lucide="archive"></i></span>
          <div>
            <h1>Gruenes Gewoelbe</h1>
            <p data-testid="active-vault">${escapeHtml(state.active_vault?.root ?? "No vault open")}</p>
          </div>
        </div>
        <span class="connection-state ${state.active_vault ? "is-active" : ""}">
          <span></span>${state.active_vault ? "Active" : "Waiting"}
        </span>
      </header>

      <div class="workbench">
        <aside class="vault-rail" aria-label="Vault navigation">
          <div class="rail-heading">
            <h2>Vaults</h2>
            <span>${state.known_vaults.length}</span>
          </div>
          <nav class="vault-list">
            ${knownVaultTemplate(state)}
          </nav>
          <div class="rail-actions">
            <button class="secondary-button" type="button" data-vault-action="open" ${state.busy ? "disabled" : ""}>
              <i data-lucide="folder-open"></i>Open Vault
            </button>
            <button class="primary-button" type="button" data-vault-action="create" ${state.busy ? "disabled" : ""}>
              <i data-lucide="folder-plus"></i>Create Vault
            </button>
          </div>
        </aside>

        <main class="workspace" aria-live="polite">
          ${messageTemplate(state)}
          ${state.repair_proposal ? repairVaultTemplate(state) : state.active_vault ? activeVaultTemplate(state.active_vault) : emptyVaultTemplate(state.busy)}
        </main>
      </div>
    </div>
  `;
}

function repairVaultTemplate(state: AppState): string {
  const proposal = state.repair_proposal;
  if (!proposal) return "";
  return `
    <section class="repair-workspace">
      <div class="workspace-icon"><i data-lucide="circle-alert"></i></div>
      <p class="eyebrow">Repair required</p>
      <h2>Repair ${escapeHtml(displayName(proposal.root))}</h2>
      <p class="repair-copy">The Vault configuration is valid, but required directories are missing. Confirm to create only this empty structure:</p>
      <ul class="repair-paths">
        ${proposal.directories.map((directory) => `<li>${escapeHtml(directory)}</li>`).join("")}
      </ul>
      <div class="empty-actions">
        <button class="secondary-button" type="button" data-repair-cancel ${state.busy ? "disabled" : ""}>Cancel</button>
        <button class="primary-button" type="button" data-repair-confirm ${state.busy ? "disabled" : ""}>Repair and Open</button>
      </div>
    </section>
  `;
}

function knownVaultTemplate(state: AppState): string {
  if (state.known_vaults.length === 0) {
    return '<p class="empty-list">No known Vaults</p>';
  }

  return state.known_vaults
    .map(
      (vault) => `
        <button class="vault-row ${state.active_vault?.root === vault.root ? "is-current" : ""}" type="button"
          data-known-vault="${escapeHtml(vault.root)}" ${state.busy ? "disabled" : ""}>
          <i data-lucide="vault"></i>
          <span>${escapeHtml(displayName(vault.root))}<small>${escapeHtml(vault.root)}</small></span>
        </button>
      `,
    )
    .join("");
}

function messageTemplate(state: AppState): string {
  const message = state.error ?? state.notice;
  if (!message) return "";
  return `
    <div class="system-message ${state.error ? "is-error" : ""}" role="alert">
      <i data-lucide="circle-alert"></i><span>${escapeHtml(message)}</span>
    </div>
  `;
}

function activeVaultTemplate(vault: ActiveVault): string {
  return `
    <section class="active-workspace">
      <div class="workspace-icon"><i data-lucide="vault"></i></div>
      <p class="eyebrow">Active Vault</p>
      <h2>${escapeHtml(displayName(vault.root))}</h2>
      <p class="vault-path">${escapeHtml(vault.root)}</p>
    </section>
  `;
}

function emptyVaultTemplate(busy: boolean): string {
  return `
    <section class="empty-workspace">
      <div class="workspace-icon"><i data-lucide="archive"></i></div>
      <p class="eyebrow">Workbench</p>
      <h2>${busy ? "Loading Vaults" : "No vault open"}</h2>
      <div class="empty-actions">
        <button class="primary-button" type="button" data-vault-action="create" ${busy ? "disabled" : ""}>
          <i data-lucide="folder-plus"></i>Create Vault
        </button>
        <button class="secondary-button" type="button" data-vault-action="open" ${busy ? "disabled" : ""}>
          <i data-lucide="folder-open"></i>Open Vault
        </button>
      </div>
    </section>
  `;
}

function displayName(path: string): string {
  const segments = path.split(/[\\/]/).filter(Boolean);
  return segments.at(-1) ?? path;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
