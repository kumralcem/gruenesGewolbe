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
  ArtworkSort,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  ImportProgress,
  ImportRunSummary,
  WorkbenchSnapshot,
} from "./contracts";

interface AppState extends DesktopStartup {
  busy: boolean;
  error: string | null;
  artwork_sort: ArtworkSort;
  workbench_snapshot: WorkbenchSnapshot | null;
  import_progress: ImportProgress | null;
  import_summary: ImportRunSummary | null;
}

export async function mountApp(root: HTMLElement, adapter: DesktopAdapter): Promise<void> {
  let state: AppState = {
    active_vault: null,
    known_vaults: [],
    repair_proposal: null,
    notice: null,
    busy: true,
    error: null,
    artwork_sort: "newest",
    workbench_snapshot: null,
    import_progress: null,
    import_summary: null,
  };

  const render = () => {
    root.innerHTML = pageTemplate(state, adapter);
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
    state = {
      ...(await adapter.startup()),
      busy: false,
      error: null,
      artwork_sort: "newest",
      workbench_snapshot: null,
      import_progress: null,
      import_summary: null,
    };
    if (state.active_vault && adapter.workbenchSnapshot) {
      state = {
        ...state,
        workbench_snapshot: await adapter.workbenchSnapshot("newest", null),
      };
    }
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
    await activateVault(() => adapter.confirmVaultRepair(proposal.root), adapter, state, update);
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

  root.querySelector<HTMLButtonElement>("[data-add-artwork]")?.addEventListener("click", async () => {
    if (!adapter.selectArtworkFiles || !adapter.addArtworkFiles) return;
    const metadata = {
      creator: inputValue(root, "[data-import-creator]"),
      year: inputValue(root, "[data-import-year]"),
      savingReason: inputValue(root, "[data-import-reason]"),
    };
    await update({ ...state, busy: true, error: null });
    try {
      const sourceFiles = await adapter.selectArtworkFiles();
      if (sourceFiles.length === 0) {
        await update({ ...state, busy: false, error: null });
        return;
      }
      await adapter.addArtworkFiles(sourceFiles, metadata);
      await refreshWorkbench(adapter, state, update, state.artwork_sort, null);
    } catch (error) {
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });

  root.querySelector<HTMLButtonElement>("[data-import-folder]")?.addEventListener("click", async () => {
    if (!adapter.selectImportFolder || !adapter.runPaintingsImport) return;
    const metadata = {
      creator: inputValue(root, "[data-import-creator]"),
      year: inputValue(root, "[data-import-year]"),
      savingReason: inputValue(root, "[data-import-reason]"),
    };
    const importExactDuplicates = inputChecked(root, "[data-import-exact]");
    try {
      const sourceFolder = await adapter.selectImportFolder();
      if (!sourceFolder) return;
      await update({
        ...state,
        busy: true,
        error: null,
        import_progress: { processed: 0, total: 0, current_file: sourceFolder },
        import_summary: null,
      });
      const summary = await adapter.runPaintingsImport(sourceFolder, {
        metadata,
        importExactDuplicates,
      }, async (progress) => {
        await update({
          ...state,
          busy: true,
          error: null,
          import_progress: progress,
          import_summary: null,
        });
      });
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, null)
        : state.workbench_snapshot;
      await update({
        ...state,
        busy: false,
        error: null,
        workbench_snapshot: snapshot,
        import_progress: null,
        import_summary: summary,
      });
    } catch (error) {
      await update({
        ...state,
        busy: false,
        error: errorMessage(error),
        import_progress: null,
      });
    }
  });

  root.querySelector<HTMLButtonElement>("[data-cancel-import]")?.addEventListener("click", async () => {
    await adapter.cancelPaintingsImport?.();
  });

  root.querySelector<HTMLSelectElement>("[data-artwork-sort]")?.addEventListener("change", async (event) => {
    const sort = (event.currentTarget as HTMLSelectElement).value as ArtworkSort;
    await refreshWorkbench(adapter, state, update, sort, state.workbench_snapshot?.selected_item?.id ?? null);
  });

  root.querySelectorAll<HTMLButtonElement>("[data-artwork-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.artworkId;
      if (!itemId) return;
      await refreshWorkbench(adapter, state, update, state.artwork_sort, itemId);
    });
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
    await activateVault(() => adapter.createVault(selected), adapter, state, update);
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
    await activateOpenedVault(result.vault, adapter, state, update);
  } catch (error) {
    await update({ ...state, busy: false, error: errorMessage(error) });
  }
}

async function activateVault(
  operation: () => Promise<ActiveVault>,
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  await update({ ...state, busy: true, error: null });
  try {
    const activeVault = await operation();
    await activateOpenedVault(activeVault, adapter, state, update);
  } catch (error) {
    await update({ ...state, busy: false, error: errorMessage(error) });
  }
}

async function activateOpenedVault(
  activeVault: ActiveVault,
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  const knownVaults = state.known_vaults.some((vault) => vault.root === activeVault.root)
    ? state.known_vaults
    : [...state.known_vaults, activeVault];
  let nextState: AppState = {
    ...state,
    active_vault: activeVault,
    known_vaults: knownVaults,
    repair_proposal: null,
    notice: null,
    busy: false,
    error: null,
  };
  if (adapter.workbenchSnapshot) {
    nextState = {
      ...nextState,
      workbench_snapshot: await adapter.workbenchSnapshot(nextState.artwork_sort, null),
    };
  }
  await update(nextState);
}

async function refreshWorkbench(
  adapter: DesktopAdapter,
  state: AppState,
  update: (state: AppState) => Promise<void>,
  sort: ArtworkSort,
  selectedItemId: string | null,
): Promise<void> {
  if (!adapter.workbenchSnapshot) return;
  await update({ ...state, artwork_sort: sort, busy: true, error: null });
  try {
    const snapshot = await adapter.workbenchSnapshot(sort, selectedItemId);
    await update({
      ...state,
      artwork_sort: sort,
      workbench_snapshot: snapshot,
      busy: false,
      error: null,
    });
  } catch (error) {
    await update({ ...state, artwork_sort: sort, busy: false, error: errorMessage(error) });
  }
}

function pageTemplate(state: AppState, adapter: DesktopAdapter): string {
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
          ${
            state.repair_proposal
              ? repairVaultTemplate(state)
              : state.active_vault && state.workbench_snapshot
                ? artworkWorkbenchTemplate(state, adapter)
                : state.active_vault
                  ? activeVaultTemplate(state.active_vault)
                  : emptyVaultTemplate(state.busy)
          }
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

function artworkWorkbenchTemplate(state: AppState, adapter: DesktopAdapter): string {
  const snapshot = state.workbench_snapshot;
  if (!snapshot) return "";
  const fileUrl = (path: string) => escapeHtml(adapter.fileUrl?.(path) ?? path);
  const selected = snapshot.selected_item;

  return `
    <section class="artwork-workspace">
      <header class="artwork-toolbar">
        <div>
          <p class="eyebrow">Artwork archive</p>
          <h2>Paintings</h2>
        </div>
        <div class="artwork-actions">
          <details class="import-metadata">
            <summary>Optional metadata</summary>
            <div>
              <label>Creator<input type="text" data-import-creator></label>
              <label>Year<input type="text" inputmode="numeric" data-import-year></label>
              <label>Saving Reason<input type="text" data-import-reason></label>
              <label><input type="checkbox" data-import-exact>Import exact duplicates anyway</label>
            </div>
          </details>
          <label class="sort-control">
            <span>Sort artwork</span>
            <select aria-label="Sort artwork" data-artwork-sort ${state.busy ? "disabled" : ""}>
              ${sortOption("newest", "Newest", state.artwork_sort)}
              ${sortOption("oldest", "Oldest", state.artwork_sort)}
              ${sortOption("title", "Title", state.artwork_sort)}
              ${sortOption("creator", "Creator", state.artwork_sort)}
              ${sortOption("year", "Year", state.artwork_sort)}
            </select>
          </label>
          <button class="primary-button" type="button" data-add-artwork ${state.busy || !adapter.addArtworkFiles || !adapter.selectArtworkFiles ? "disabled" : ""}>
            <i data-lucide="folder-plus"></i>Add Artwork
          </button>
          <button class="secondary-button" type="button" data-import-folder ${state.busy || !adapter.selectImportFolder || !adapter.runPaintingsImport ? "disabled" : ""}>
            <i data-lucide="folder-open"></i>Import Folder
          </button>
        </div>
      </header>

      ${importRunTemplate(state, adapter)}

      <div class="artwork-content ${selected ? "has-selection" : ""}">
        <div class="artwork-gallery" aria-label="Artwork gallery">
          ${
            snapshot.artwork_items.length === 0
              ? '<p class="gallery-empty">No artwork saved yet. Add one or more image files to begin.</p>'
              : snapshot.artwork_items
                  .map(
                    (item) => `
                      <button class="artwork-card ${selected?.id === item.id ? "is-selected" : ""}" type="button"
                        data-artwork-id="${escapeHtml(item.id)}" ${state.busy ? "disabled" : ""}>
                        <span class="artwork-preview ${item.thumbnail_is_placeholder ? "is-placeholder" : ""}">
                          <img src="${fileUrl(item.thumbnail_file)}" alt="${escapeHtml(item.title)}">
                        </span>
                        <span class="artwork-caption">
                          <strong>${escapeHtml(item.title)}</strong>
                          <small>${escapeHtml(metadataLine(item.creator, item.year))}</small>
                        </span>
                      </button>
                    `,
                  )
                  .join("")
          }
        </div>

        ${
          selected
            ? `
              <aside class="artwork-details" aria-label="Artwork details">
                <img class="primary-file" src="${fileUrl(selected.primary_file)}" alt="Primary File for ${escapeHtml(selected.title)}">
                <p class="eyebrow">Primary file</p>
                <h2>${escapeHtml(selected.title)}</h2>
                <p class="detail-byline">${escapeHtml(metadataLine(selected.creator, selected.year))}</p>
                <dl>
                  <div><dt>Home Subvault</dt><dd>${escapeHtml(selected.home_subvault)}</dd></div>
                  <div><dt>Review Status</dt><dd>${escapeHtml(selected.review_status)}</dd></div>
                  ${selected.saving_reason ? `<div><dt>Saving Reason</dt><dd>${escapeHtml(selected.saving_reason)}</dd></div>` : ""}
                  <div><dt>File</dt><dd class="file-path">${escapeHtml(selected.primary_file)}</dd></div>
                </dl>
              </aside>
            `
            : ""
        }
      </div>
    </section>
  `;
}

function importRunTemplate(state: AppState, adapter: DesktopAdapter): string {
  if (state.import_progress) {
    const progress = state.import_progress;
    return `
      <section class="import-run-status" aria-label="Import progress">
        <div>
          <p class="eyebrow">Import Run</p>
          <strong>${progress.processed} of ${progress.total || "?"} files processed</strong>
          <small>${escapeHtml(displayName(progress.current_file))}</small>
        </div>
        <button class="secondary-button" type="button" data-cancel-import ${adapter.cancelPaintingsImport ? "" : "disabled"}>Cancel Import</button>
      </section>
    `;
  }
  if (!state.import_summary) return "";
  const summary = state.import_summary;
  return `
    <section class="import-run-status is-summary" aria-label="Import summary">
      <div>
        <p class="eyebrow">Import Run</p>
        <h3>${summary.cancelled ? "Import cancelled" : "Import complete"}</h3>
        <div class="summary-counts">
          <span>${summary.imported_count} imported</span>
          <span>${summary.skipped_count} skipped</span>
          <span>${summary.duplicate_candidate_count} duplicate-candidate</span>
          <span>${summary.exact_duplicate_count} exact duplicate</span>
          <span>${summary.cancelled_count} cancelled</span>
          <span>${summary.failed_count} failed</span>
        </div>
        <div class="summary-details">
          ${summaryGroup(
            "Skipped",
            summary.skipped_entries.map((entry) => `${displayName(entry.path)} — ${entry.reason}`),
          )}
          ${summaryGroup(
            "Duplicate candidates",
            summary.duplicate_candidate_entries.map(
              (entry) =>
                `${displayName(entry.path)} — item=${entry.item_id} candidates=${entry.candidate_count}`,
            ),
          )}
          ${summaryGroup("Cancelled", summary.cancelled_files.map(displayName))}
          ${summaryGroup(
            "Failed",
            summary.failed_entries.map((entry) => `${displayName(entry.path)} — ${entry.error}`),
          )}
          ${summaryGroup("Maintenance", summary.maintenance_errors)}
          ${summaryGroup(
            "Vault problems",
            summary.vault_problems.map((problem) => `${displayName(problem.path)} — ${problem.error}`),
          )}
        </div>
      </div>
    </section>
  `;
}

function summaryGroup(label: string, entries: string[]): string {
  if (entries.length === 0) return "";
  return `
    <div>
      <strong>${label}</strong>
      <ul>${entries.map((entry) => `<li>${escapeHtml(entry)}</li>`).join("")}</ul>
    </div>
  `;
}

function sortOption(value: ArtworkSort, label: string, selected: ArtworkSort): string {
  return `<option value="${value}" ${value === selected ? "selected" : ""}>${label}</option>`;
}

function metadataLine(creator: string, year: string): string {
  return [creator, year].filter(Boolean).join(" · ") || "Unknown creator";
}

function inputValue(root: HTMLElement, selector: string): string | null {
  const value = root.querySelector<HTMLInputElement>(selector)?.value.trim();
  return value || null;
}

function inputChecked(root: HTMLElement, selector: string): boolean {
  return root.querySelector<HTMLInputElement>(selector)?.checked ?? false;
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
