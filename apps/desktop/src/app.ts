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
  ArtworkImportOutcome,
  ArtworkSort,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  ImportProgress,
  ItemDetails,
  ItemRecordEdit,
  WorkbenchSnapshot,
} from "./contracts";

interface AppState extends DesktopStartup {
  busy: boolean;
  error: string | null;
  artwork_sort: ArtworkSort;
  search_query: string;
  workbench_snapshot: WorkbenchSnapshot | null;
  import_progress: ImportProgress | null;
  import_summary: ArtworkImportOutcome | null;
  import_summary_kind: "folder" | "selected" | null;
  pending_item_edit: ItemRecordEdit | null;
  item_record_conflict: ItemDetails | null;
  selected_review_reason_id?: string | null;
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
    search_query: "",
    workbench_snapshot: null,
    import_progress: null,
    import_summary: null,
    import_summary_kind: null,
    pending_item_edit: null,
    item_record_conflict: null,
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
      search_query: "",
      workbench_snapshot: null,
      import_progress: null,
      import_summary: null,
      import_summary_kind: null,
      pending_item_edit: null,
      item_record_conflict: null,
    };
    if (state.active_vault && adapter.workbenchSnapshot) {
      state = {
        ...state,
        workbench_snapshot: await adapter.workbenchSnapshot("newest", null, null),
      };
    }
  } catch (error) {
    state = { ...state, busy: false, error: errorMessage(error) };
  }
  render();

  window.addEventListener("focus", () => {
    if (!state.active_vault || state.busy || !adapter.workbenchSnapshot) return;
    const form = root.querySelector<HTMLFormElement>("[data-item-record-form]");
    const selected = state.workbench_snapshot?.selected_item;
    const draft = form && selected ? itemRecordEdit(form, selected, false) : null;
    const stateForRefresh = {
      ...state,
      pending_item_edit: draft && selected && itemRecordEditIsDirty(draft, selected) ? draft : null,
    };
    void refreshWorkbench(
      adapter,
      stateForRefresh,
      async (nextState) => {
        state = nextState;
        render();
      },
      state.artwork_sort,
      state.workbench_snapshot?.selected_item?.id ?? null,
      true,
    );
  });
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
    const importExactDuplicates = inputChecked(root, "[data-import-exact]");
    await update({ ...state, busy: true, error: null });
    try {
      const sourceFiles = await adapter.selectArtworkFiles();
      if (sourceFiles.length === 0) {
        await update({ ...state, busy: false, error: null });
        return;
      }
      const summary = await adapter.addArtworkFiles(sourceFiles, {
        metadata,
        importExactDuplicates,
      });
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, null, state.search_query)
        : state.workbench_snapshot;
      await update({
        ...state,
        busy: false,
        error: null,
        workbench_snapshot: snapshot,
        import_summary: summary,
        import_summary_kind: "selected",
      });
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
        import_summary_kind: "folder",
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
          import_summary_kind: "folder",
        });
      });
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, null, state.search_query)
        : state.workbench_snapshot;
      await update({
        ...state,
        busy: false,
        error: null,
        workbench_snapshot: snapshot,
        import_progress: null,
        import_summary: summary,
        import_summary_kind: "folder",
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

  root.querySelector<HTMLFormElement>("[data-vault-search]")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const query = inputValue(root, "[data-search-query]") ?? "";
    await refreshWorkbench(
      adapter,
      { ...state, search_query: query },
      update,
      state.artwork_sort,
      null,
    );
  });

  root.querySelectorAll<HTMLButtonElement>("[data-search-result-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.searchResultId;
      if (!itemId) return;
      await refreshWorkbench(adapter, state, update, state.artwork_sort, itemId);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-artwork-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.artworkId;
      if (!itemId) return;
      await refreshWorkbench(adapter, state, update, state.artwork_sort, itemId);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-review-item-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.reviewItemId;
      const reasonId = button.dataset.reviewReasonId;
      if (!itemId || !reasonId) return;
      await refreshWorkbench(
        adapter,
        { ...state, selected_review_reason_id: reasonId },
        update,
        state.artwork_sort,
        itemId,
      );
      const reason = Array.from(root.querySelectorAll<HTMLElement>("[data-review-reason]")).find(
        (element) => element.dataset.reviewReason === reasonId,
      );
      reason?.scrollIntoView({ block: "nearest" });
      reason?.focus();
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-review-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const selected = state.workbench_snapshot?.selected_item;
      const reasonId = button.dataset.reviewReasonId;
      const action = button.dataset.reviewAction as "accept" | "correct" | "dismiss";
      if (!selected || !reasonId || !adapter.resolveReviewReason) return;
      const reasonCard = button.closest<HTMLElement>("[data-review-reason]");
      const correction =
        action === "correct"
          ? reasonCard?.querySelector<HTMLInputElement>("[data-review-correction]")?.value.trim() || null
          : null;
      if (action === "correct" && !correction) return;
      try {
        await adapter.resolveReviewReason({
          item_id: selected.id,
          reason_id: reasonId,
          expected_revision: selected.record_revision,
          action,
          correction,
        });
        await refreshWorkbench(adapter, state, update, state.artwork_sort, selected.id);
      } catch (error) {
        await update({ ...state, error: errorMessage(error) });
      }
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-duplicate-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const selected = state.workbench_snapshot?.selected_item;
      const reasonId = button.dataset.reviewReasonId;
      const action = button.dataset.duplicateAction as "not-a-duplicate" | "keep-both" | "move-this-item-to-vault-trash";
      if (!selected || !reasonId || !adapter.resolveDuplicateCandidate) return;
      try {
        await adapter.resolveDuplicateCandidate({ item_id: selected.id, reason_id: reasonId, expected_revision: selected.record_revision, action });
        await refreshWorkbench(adapter, state, update, state.artwork_sort, action === "move-this-item-to-vault-trash" ? null : selected.id, true);
      } catch (error) { await update({ ...state, error: errorMessage(error) }); }
    });
  });

  root.querySelector<HTMLButtonElement>("[data-refresh-records]")?.addEventListener("click", async () => {
    await refreshWorkbench(
      adapter,
      { ...state, pending_item_edit: null, item_record_conflict: null },
      update,
      state.artwork_sort,
      state.workbench_snapshot?.selected_item?.id ?? null,
      true,
    );
  });

  root.querySelector<HTMLButtonElement>("[data-open-activity-log]")?.addEventListener("click", async () => {
    if (!adapter.openActivityLog) return;
    try {
      await adapter.openActivityLog();
    } catch (error) {
      await update({ ...state, error: errorMessage(error) });
    }
  });

  root.querySelector<HTMLButtonElement>("[data-save-item-record]")?.addEventListener("click", async () => {
    const form = root.querySelector<HTMLFormElement>("[data-item-record-form]");
    const selected = state.workbench_snapshot?.selected_item;
    if (!form || !selected || !adapter.saveItemRecord || !form.reportValidity()) return;
    await saveItemEdit(adapter, state, itemRecordEdit(form, selected, false), update);
  });

  root.querySelector<HTMLButtonElement>("[data-conflict-reload]")?.addEventListener("click", async () => {
    const external = state.item_record_conflict;
    if (!external || !state.workbench_snapshot) return;
    await update({
      ...state,
      workbench_snapshot: { ...state.workbench_snapshot, selected_item: external },
      pending_item_edit: null,
      item_record_conflict: null,
      error: null,
    });
  });

  root.querySelector<HTMLButtonElement>("[data-conflict-overwrite]")?.addEventListener("click", async () => {
    if (!state.pending_item_edit || !adapter.saveItemRecord) return;
    await saveItemEdit(
      adapter,
      state,
      {
        ...state.pending_item_edit,
        expected_revision: state.item_record_conflict?.record_revision ?? state.pending_item_edit.expected_revision,
        overwrite_conflict: true,
      },
      update,
    );
  });

  root.querySelector<HTMLButtonElement>("[data-confirm-folder-rename]")?.addEventListener("click", async () => {
    const selected = state.workbench_snapshot?.selected_item;
    if (!selected?.folder_rename_proposal || !adapter.confirmItemFolderRename) return;
    await update({ ...state, busy: true, error: null });
    try {
      await adapter.confirmItemFolderRename(selected.id, selected.folder_rename_proposal);
      await refreshWorkbench(adapter, state, update, state.artwork_sort, selected.id);
    } catch (error) {
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });

  root.querySelector<HTMLButtonElement>("[data-move-to-trash]")?.addEventListener("click", async () => {
    const selected = state.workbench_snapshot?.selected_item;
    if (!selected || !adapter.moveItemToTrash) return;
    try { await adapter.moveItemToTrash(selected.id); await refreshWorkbench(adapter, state, update, state.artwork_sort, null, true); }
    catch (error) { await update({ ...state, error: errorMessage(error) }); }
  });
  root.querySelectorAll<HTMLButtonElement>("[data-restore-trash-id]").forEach((button) => button.addEventListener("click", async () => {
    const id = button.dataset.restoreTrashId; if (!id || !adapter.restoreTrashedItem) return;
    try { await adapter.restoreTrashedItem(id); await refreshWorkbench(adapter, state, update, state.artwork_sort, id, true); }
    catch (error) { await update({ ...state, error: errorMessage(error) }); }
  }));
  root.querySelectorAll<HTMLButtonElement>("[data-permanently-delete-trash-id]").forEach((button) => button.addEventListener("click", async () => {
    const id = button.dataset.permanentlyDeleteTrashId;
    const input = button.parentElement?.querySelector<HTMLInputElement>("[data-delete-confirmation]");
    if (!id || !input || !adapter.permanentlyDeleteTrashedItem) return;
    try { await adapter.permanentlyDeleteTrashedItem(id, input.value); await refreshWorkbench(adapter, state, update, state.artwork_sort, null, true); }
    catch (error) { await update({ ...state, error: errorMessage(error) }); }
  }));
}

async function saveItemEdit(
  adapter: DesktopAdapter,
  state: AppState,
  edit: ItemRecordEdit,
  update: (state: AppState) => Promise<void>,
): Promise<void> {
  if (!adapter.saveItemRecord) return;
  try {
    const result = await adapter.saveItemRecord(edit);
    if (result.status === "conflict") {
      await update({
        ...state,
        pending_item_edit: edit,
        item_record_conflict: result.external_item,
        error: null,
      });
      return;
    }
    const snapshot = adapter.workbenchSnapshot
      ? await adapter.workbenchSnapshot(state.artwork_sort, edit.id, state.search_query)
      : state.workbench_snapshot
        ? { ...state.workbench_snapshot, selected_item: result.item }
        : null;
    await update({
      ...state,
      workbench_snapshot: snapshot,
      pending_item_edit: null,
      item_record_conflict: null,
      error: null,
    });
  } catch (error) {
    await update({ ...state, error: errorMessage(error) });
  }
}

function itemRecordEdit(
  form: HTMLFormElement,
  selected: ItemDetails,
  overwriteConflict: boolean,
): ItemRecordEdit {
  const data = new FormData(form);
  return {
    id: selected.id,
    expected_revision: selected.record_revision,
    overwrite_conflict: overwriteConflict,
    title: String(data.get("title") ?? "").trim(),
    creator: String(data.get("creator") ?? "").trim(),
    year: String(data.get("year") ?? "").trim(),
    saving_reason: String(data.get("saving_reason") ?? "").trim(),
    summary: String(data.get("summary") ?? "").trim(),
    tags: String(data.get("tags") ?? "")
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean),
  };
}

function itemRecordEditIsDirty(edit: ItemRecordEdit, selected: ItemDetails): boolean {
  return (
    edit.title !== selected.title ||
    edit.creator !== selected.creator ||
    edit.year !== selected.year ||
    edit.saving_reason !== (selected.saving_reason ?? "") ||
    edit.summary !== (selected.summary ?? "") ||
    edit.tags.join("\n") !== selected.tags.join("\n")
  );
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
      workbench_snapshot: await adapter.workbenchSnapshot(
        nextState.artwork_sort,
        null,
        nextState.search_query,
      ),
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
  refreshRecords = false,
): Promise<void> {
  if (!adapter.workbenchSnapshot) return;
  await update({ ...state, artwork_sort: sort, busy: true, error: null });
  try {
    const snapshot =
      refreshRecords && adapter.refreshWorkbenchSnapshot
        ? await adapter.refreshWorkbenchSnapshot(sort, selectedItemId, state.search_query)
        : await adapter.workbenchSnapshot(sort, selectedItemId, state.search_query);
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
          <button class="secondary-button" type="button" data-refresh-records ${state.busy || !adapter.workbenchSnapshot ? "disabled" : ""}>
            Refresh Item Records
          </button>
        </div>
      </header>

      ${importRunTemplate(state, adapter)}

      <section class="vault-tools" aria-label="Vault tools">
        <form class="vault-search" data-vault-search>
          <label>Search Active Vault<input type="search" data-search-query value="${escapeHtml(state.search_query)}"></label>
          <button class="secondary-button" type="submit" ${state.busy ? "disabled" : ""}>Search</button>
        </form>
        <button class="secondary-button" type="button" data-open-activity-log ${state.busy || !adapter.openActivityLog ? "disabled" : ""}>Open Activity Log</button>
      </section>

      ${vaultProblemsTemplate(snapshot)}
      ${searchResultsTemplate(snapshot, state)}

      ${reviewQueueTemplate(snapshot, state)}
      ${vaultTrashTemplate(snapshot, state, adapter)}

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
                ${itemRecordConflictTemplate(state)}
                ${reviewReasonsTemplate(state, selected, adapter)}
                ${itemRecordEditorTemplate(state, selected, adapter)}
                ${folderRenameTemplate(selected, adapter)}
                <button class="danger-button" type="button" data-move-to-trash ${adapter.moveItemToTrash ? "" : "disabled"}>Move to Vault Trash</button>
                <dl class="record-context">
                  <div><dt>Home Subvault</dt><dd>${escapeHtml(selected.home_subvault)}</dd></div>
                  <div><dt>Review Status</dt><dd>${escapeHtml(selected.review_status)}</dd></div>
                  <div><dt>Collections</dt><dd>${escapeHtml(selected.collections.join(", ") || "None")}</dd></div>
                  <div><dt>Item Links</dt><dd>${itemLinksTemplate(selected)}</dd></div>
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

function vaultTrashTemplate(snapshot: WorkbenchSnapshot, state: AppState, adapter: DesktopAdapter): string {
  const items = snapshot.trashed_items ?? [];
  return `<section class="vault-trash" aria-label="Vault Trash"><p class="eyebrow">Recoverable removal</p><h2>Vault Trash</h2>${items.length === 0 ? "<p>Vault Trash is empty.</p>" : items.map(item => `<article><h3>${escapeHtml(item.title)}</h3><p>${escapeHtml(metadataLine(item.creator ?? "", item.year ?? ""))} · ${escapeHtml(item.home_subvault)}</p><p>Review: ${escapeHtml(item.review_status ?? "Unknown")} · Tags: ${escapeHtml(item.tags?.join(", ") || "None")}</p><p class="file-path">${escapeHtml(item.item_folder)}</p><p>Collections: ${escapeHtml(item.collections.join(", ") || "None")} (target in Vault Trash)</p><p>Incoming Item Links: ${escapeHtml(item.incoming_item_links.map(link => `${link.label} (${link.source_item_id}) → target in Vault Trash`).join(", ") || "None")}</p><button class="secondary-button" type="button" data-restore-trash-id="${escapeHtml(item.id)}" ${state.busy || !adapter.restoreTrashedItem ? "disabled" : ""}>Restore</button><div><label>Type stable item ID <code>${escapeHtml(item.id)}</code> to permanently delete<input data-delete-confirmation aria-label="Confirm permanent deletion for ${escapeHtml(item.title)}"></label><button class="danger-button" type="button" data-permanently-delete-trash-id="${escapeHtml(item.id)}" ${state.busy || !adapter.permanentlyDeleteTrashedItem ? "disabled" : ""}>Permanently Delete</button></div></article>`).join("")}</section>`;
}

function vaultProblemsTemplate(snapshot: WorkbenchSnapshot): string {
  if (snapshot.vault_problems.length === 0) return "";
  return `
    <section class="vault-problems" aria-label="Vault Problems">
      <p class="eyebrow">Localized file errors</p>
      <h2>Vault Problems</h2>
      <ul>
        ${snapshot.vault_problems
          .map(
            (problem) =>
              `<li><strong>${escapeHtml(displayName(problem.path))}</strong><span class="file-path">${escapeHtml(problem.path)}</span><small>${escapeHtml(problem.error)}</small></li>`,
          )
          .join("")}
      </ul>
    </section>`;
}

function searchResultsTemplate(snapshot: WorkbenchSnapshot, state: AppState): string {
  if (!state.search_query) return "";
  return `
    <section class="search-results" aria-label="Search Results">
      <p class="eyebrow">Search Results</p>
      ${
        snapshot.search_results.length === 0
          ? `<p>No Saved Items match “${escapeHtml(state.search_query)}”.</p>`
          : `<div>${snapshot.search_results
              .map(
                (result) =>
                  `<button type="button" data-search-result-id="${escapeHtml(result.id)}" ${state.busy ? "disabled" : ""}>${escapeHtml(result.title)}</button>`,
              )
              .join("")}</div>`
      }
    </section>`;
}

function reviewQueueTemplate(snapshot: WorkbenchSnapshot, state: AppState): string {
  if (snapshot.review_queue.length === 0) return "";
  return `
    <section class="review-queue" aria-label="Review Queue">
      <p class="eyebrow">Review Queue</p>
      <div class="review-queue-items">
        ${snapshot.review_queue
          .flatMap((item) =>
            item.review_reasons.map(
              (reason) => `
                <button type="button" data-review-item-id="${escapeHtml(item.id)}"
                  data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy ? "disabled" : ""}>
                  <strong>${escapeHtml(item.title)}</strong>
                  <span>${escapeHtml(reason.message)}</span>
                </button>`,
            ),
          )
          .join("")}
      </div>
    </section>`;
}

function reviewReasonsTemplate(
  state: AppState,
  selected: ItemDetails,
  adapter: DesktopAdapter,
): string {
  if (selected.review_reasons.length === 0) {
    return '<section class="review-reasons"><strong>Reviewed</strong><p>No unresolved Review Reasons.</p></section>';
  }
  return `
    <section class="review-reasons" aria-label="Review Reasons">
      <strong>Review Reasons</strong>
      ${selected.review_reasons
        .map(
          (reason) => reason.kind === "duplicate-candidate" ? duplicateCandidateTemplate(state, reason, adapter) : `
            <article class="review-reason ${state.selected_review_reason_id === reason.id ? "is-targeted" : ""}" data-review-reason="${escapeHtml(reason.id)}" tabindex="-1">
              <h3>${escapeHtml(reason.message)}</h3>
              <p>${escapeHtml(reason.evidence)}</p>
              <label>${reason.target_field ? `Correct ${escapeHtml(reason.target_field)}` : "Describe the correction"}
                <input data-review-correction value="">
              </label>
              <div class="review-reason-actions">
                <button class="secondary-button" type="button" data-review-action="accept" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveReviewReason ? "disabled" : ""}>Accept</button>
                <button class="secondary-button" type="button" data-review-action="correct" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveReviewReason ? "disabled" : ""}>Correct</button>
                <button class="secondary-button" type="button" data-review-action="dismiss" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveReviewReason ? "disabled" : ""}>Dismiss</button>
              </div>
            </article>`,
        )
        .join("")}
    </section>`;
}

function duplicateCandidateTemplate(state: AppState, reason: ItemDetails["review_reasons"][number], adapter: DesktopAdapter): string {
  const candidateId = reason.candidate_item_id;
  const candidate = state.workbench_snapshot?.artwork_items.find((item) => item.id === candidateId);
  const ideaCandidate = state.workbench_snapshot?.idea_sources.find((item) => item.id === candidateId);
  const comparison = candidate
    ? `<dl class="duplicate-comparison"><div><dt>Candidate</dt><dd>${escapeHtml(candidate.title)}</dd></div><div><dt>Creator</dt><dd>${escapeHtml(candidate.creator)}</dd></div><div><dt>Year</dt><dd>${escapeHtml(candidate.year)}</dd></div><div><dt>Saved Item</dt><dd>${escapeHtml(candidate.id)}</dd></div></dl>`
    : ideaCandidate
      ? `<dl class="duplicate-comparison"><div><dt>Candidate</dt><dd>${escapeHtml(ideaCandidate.title)}</dd></div><div><dt>Source Link</dt><dd>${escapeHtml(ideaCandidate.source_link)}</dd></div><div><dt>Saved Item</dt><dd>${escapeHtml(ideaCandidate.id)}</dd></div></dl>`
    : `<p>Candidate Saved Item: ${escapeHtml(candidateId ?? reason.evidence.split(":")[0] ?? "Unknown")}</p>`;
  return `<article class="review-reason ${state.selected_review_reason_id === reason.id ? "is-targeted" : ""}" data-review-reason="${escapeHtml(reason.id)}" tabindex="-1">
    <h3>${escapeHtml(reason.message)}</h3><p>Matching evidence: ${escapeHtml(reason.evidence)}</p>${comparison}
    <div class="review-reason-actions">
      <button class="secondary-button" type="button" data-duplicate-action="not-a-duplicate" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveDuplicateCandidate ? "disabled" : ""}>Not a Duplicate</button>
      <button class="secondary-button" type="button" data-duplicate-action="keep-both" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveDuplicateCandidate ? "disabled" : ""}>Keep Both</button>
      <button class="danger-button" type="button" data-duplicate-action="move-this-item-to-vault-trash" data-review-reason-id="${escapeHtml(reason.id)}" ${state.busy || !adapter.resolveDuplicateCandidate ? "disabled" : ""}>Move This Item to Vault Trash</button>
    </div></article>`;
}

function itemRecordEditorTemplate(
  state: AppState,
  selected: ItemDetails,
  adapter: DesktopAdapter,
): string {
  const pending = state.pending_item_edit?.id === selected.id ? state.pending_item_edit : null;
  const value = (field: "title" | "creator" | "year" | "saving_reason" | "summary") =>
    pending?.[field] ?? selected[field] ?? "";
  const tags = pending?.tags ?? selected.tags;
  return `
    <form class="item-record-form" data-item-record-form>
      <label>Title<input name="title" value="${escapeHtml(value("title"))}" required></label>
      <div class="field-pair">
        <label>Creator<input name="creator" value="${escapeHtml(value("creator"))}" required></label>
        <label>Year<input name="year" value="${escapeHtml(value("year"))}" pattern="(?:\\d{4}|Unknown Year)" required></label>
      </div>
      <label>Saving Reason<textarea name="saving_reason">${escapeHtml(value("saving_reason"))}</textarea></label>
      <label>Summary<textarea name="summary">${escapeHtml(value("summary"))}</textarea></label>
      <label>Tags<input name="tags" value="${escapeHtml(tags.join(", "))}" aria-describedby="tag-hint"></label>
      <small id="tag-hint">Comma-separated; known aliases normalize through the Tag Registry.</small>
      <button class="primary-button" type="button" data-save-item-record ${state.busy || !adapter.saveItemRecord ? "disabled" : ""}>Save Item Record</button>
    </form>
  `;
}

function itemRecordConflictTemplate(state: AppState): string {
  const conflict = state.item_record_conflict;
  if (!conflict) return "";
  return `
    <section class="item-record-conflict" role="alert">
      <strong>Item Record Conflict</strong>
      <p>The file changed after this editor loaded. Review the external known fields before choosing an action.</p>
      <dl>
        <div><dt>Title</dt><dd>${escapeHtml(conflict.title)}</dd></div>
        <div><dt>Creator</dt><dd>${escapeHtml(conflict.creator)}</dd></div>
        <div><dt>Year</dt><dd>${escapeHtml(conflict.year)}</dd></div>
        <div><dt>Saving Reason</dt><dd>${escapeHtml(conflict.saving_reason ?? "None")}</dd></div>
        <div><dt>Summary</dt><dd>${escapeHtml(conflict.summary ?? "None")}</dd></div>
        <div><dt>Tags</dt><dd>${escapeHtml(conflict.tags.join(", ") || "None")}</dd></div>
      </dl>
      <div class="conflict-actions">
        <button class="secondary-button" type="button" data-conflict-reload>Reload External Version</button>
        <button class="danger-button" type="button" data-conflict-overwrite>Overwrite After Review</button>
      </div>
    </section>
  `;
}

function folderRenameTemplate(selected: ItemDetails, adapter: DesktopAdapter): string {
  const proposal = selected.folder_rename_proposal;
  if (!proposal) return "";
  return `
    <section class="folder-rename-proposal">
      <strong>Folder rename available</strong>
      <dl>
        <div><dt>Current</dt><dd class="file-path">${escapeHtml(proposal.current_path)}</dd></div>
        <div><dt>Proposed</dt><dd class="file-path">${escapeHtml(proposal.proposed_path)}</dd></div>
      </dl>
      <button class="secondary-button" type="button" data-confirm-folder-rename ${adapter.confirmItemFolderRename ? "" : "disabled"}>Confirm Folder Rename</button>
    </section>
  `;
}

function itemLinksTemplate(selected: ItemDetails): string {
  if (selected.item_links.length === 0) return "None";
  return selected.item_links
    .map(
      (link) =>
        `<span class="item-link"><strong>${escapeHtml(link.label)}</strong> <small>${escapeHtml(link.link_type)} · ${escapeHtml(link.target)}${link.target_in_vault_trash ? " · In Vault Trash" : ""}</small></span>`,
    )
    .join("");
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
  const summaryLabel = state.import_summary_kind === "selected" ? "Selected Files" : "Import Run";
  return `
    <section class="import-run-status is-summary" aria-label="Import summary">
      <div>
        <p class="eyebrow">${summaryLabel}</p>
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
