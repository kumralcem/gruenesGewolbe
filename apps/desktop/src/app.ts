import {
  Archive,
  BookOpen,
  CircleAlert,
  FolderOpen,
  FolderPlus,
  Images,
  Vault,
  createIcons,
} from "lucide";

import type {
  ActiveVault,
  ArtworkGridItem,
  ArtworkImportOutcome,
  ArtworkSort,
  SourceLinkCaptureResult,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  ImportProgress,
  IdeaSourceContent,
  ItemDetails,
  ItemRecordEdit,
  OpenAiProviderStatus,
  ArtworkEnrichmentProgress,
  WorkbenchSnapshot,
} from "./contracts";

interface AppState extends DesktopStartup {
  active_view: "paintings" | "idea-sources" | "settings";
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
  thumbnail_remaining: number | null;
  pending_selected_artwork_id: string | null;
  selected_artwork_error: string | null;
  capture_open: boolean;
  capture_fallback: Extract<SourceLinkCaptureResult, { status: "needs_manual_fallback" }> | null;
  capture_pasted_image: { fileName: string; bytes: number[] } | null;
  capture_copied_text: string;
  capture_source_link: string;
  capture_title: string;
  capture_saving_reason: string;
  idea_source_reading: IdeaSourceContent | null;
  summary_provider: OpenAiProviderStatus | null;
  summary_notice: string | null;
  enrichment_progress: ArtworkEnrichmentProgress | null;
}

type StateUpdate = (state: AppState | ((current: AppState) => AppState)) => Promise<void>;
interface ArtworkSelection {
  select(itemId: string): Promise<void>;
  cancel(): void;
}

export async function mountApp(root: HTMLElement, adapter: DesktopAdapter): Promise<void> {
  const thumbnailSnapshotBatchSize = 8;
  let state: AppState = {
    active_view: "paintings",
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
    thumbnail_remaining: null,
    pending_selected_artwork_id: null,
    selected_artwork_error: null,
    capture_open: false,
    capture_fallback: null,
    capture_pasted_image: null,
    capture_copied_text: "",
    capture_source_link: "",
    capture_title: "",
    capture_saving_reason: "",
    idea_source_reading: null,
    summary_provider: null,
    summary_notice: null,
    enrichment_progress: null,
  };

  let thumbnailPreparationRunning = false;
  let selectionRequest = 0;
  const requestEpoch = { value: 0 };

  const update: StateUpdate = async (nextState) => {
    selectionRequest += 1;
    state = typeof nextState === "function" ? nextState(state) : nextState;
    render();
  };
  const updateSelection: StateUpdate = async (incomingState) => {
    const nextState = typeof incomingState === "function" ? incomingState(state) : incomingState;
    state = {
      ...state,
      workbench_snapshot: nextState.workbench_snapshot,
      error: nextState.error,
      selected_review_reason_id: nextState.selected_review_reason_id,
      pending_item_edit: nextState.pending_item_edit,
      item_record_conflict: nextState.item_record_conflict,
      pending_selected_artwork_id: nextState.pending_selected_artwork_id,
      selected_artwork_error: nextState.selected_artwork_error,
      idea_source_reading: nextState.idea_source_reading,
      summary_notice: nextState.summary_notice,
      enrichment_progress: nextState.enrichment_progress,
    };
    patchArtworkSelection(root, adapter, state, update, updateSelection, artworkSelection, requestEpoch);
  };
  const selectArtwork = async (itemId: string) => {
    if (!adapter.workbenchSnapshot) return;
    const request = ++selectionRequest;
    await updateSelection({
      ...state,
      pending_selected_artwork_id: itemId,
      selected_artwork_error: null,
      error: null,
    });
    try {
      const snapshot = adapter.getItemDetails && state.workbench_snapshot
        ? {
            ...state.workbench_snapshot,
            selected_item: await adapter.getItemDetails(itemId),
          }
        : await adapter.workbenchSnapshot(
            state.artwork_sort,
            itemId,
            state.search_query,
          );
      if (request !== selectionRequest) return;
      await updateSelection({
        ...state,
        workbench_snapshot: snapshot,
        pending_selected_artwork_id: null,
        selected_artwork_error: null,
        error: null,
      });
    } catch (error) {
      if (request !== selectionRequest) return;
      await updateSelection({
        ...state,
        pending_selected_artwork_id: itemId,
        selected_artwork_error: errorMessage(error),
        error: null,
      });
    }
  };
  const artworkSelection: ArtworkSelection = {
    select: selectArtwork,
    cancel: () => { selectionRequest += 1; },
  };

  const render = () => {
    const workspaceScrollTop = root.querySelector<HTMLElement>(".workspace")?.scrollTop ?? 0;
    root.innerHTML = pageTemplate(state, adapter);
    createIcons({
      icons: { Archive, BookOpen, CircleAlert, FolderOpen, FolderPlus, Images, Vault },
      attrs: { "aria-hidden": "true", width: 18, height: 18 },
    });
    bindActions(root, adapter, state, update, updateSelection, artworkSelection, requestEpoch);
    const workspace = root.querySelector<HTMLElement>(".workspace");
    if (workspace) workspace.scrollTop = workspaceScrollTop;
    scheduleThumbnailPreparation();
  };

  const scheduleThumbnailPreparation = () => {
    if (
      thumbnailPreparationRunning ||
      state.pending_selected_artwork_id !== null ||
      !state.active_vault ||
      !state.workbench_snapshot ||
      !adapter.prepareThumbnailPreviews ||
      !adapter.workbenchSnapshot
    ) return;
    thumbnailPreparationRunning = true;
    void (async () => {
      let generatedSinceSnapshot = 0;
      try {
        while (state.active_vault) {
          const prepared = await adapter.prepareThumbnailPreviews!(1);
          state = { ...state, thumbnail_remaining: prepared.remaining, error: null };
          generatedSinceSnapshot += prepared.generated;
          const refreshWorkbench =
            generatedSinceSnapshot >= thumbnailSnapshotBatchSize ||
            prepared.remaining === 0 ||
            prepared.generated === 0;
          if (refreshWorkbench && generatedSinceSnapshot > 0) {
            const snapshotVaultRoot = state.active_vault?.root;
            if (!snapshotVaultRoot) break;
            const refreshedSnapshot = await adapter.workbenchSnapshot!(
              state.artwork_sort,
              state.workbench_snapshot?.selected_item?.id ?? null,
              state.search_query,
            );
            generatedSinceSnapshot = 0;
            const currentSnapshot = state.workbench_snapshot;
            if (
              state.active_vault?.root === snapshotVaultRoot
              && currentSnapshot?.active_vault.root === snapshotVaultRoot
            ) {
              const refreshedArtwork = new Map(
                refreshedSnapshot.artwork_items.map((item) => [item.id, item]),
              );
              const mergedSnapshot = {
                ...currentSnapshot,
                artwork_items: currentSnapshot.artwork_items.map((item) => {
                    const refreshed = refreshedArtwork.get(item.id);
                    return refreshed
                      ? {
                          ...item,
                          thumbnail_file: refreshed.thumbnail_file,
                          thumbnail_is_placeholder: refreshed.thumbnail_is_placeholder,
                        }
                      : item;
                  }),
              };
              state = {
                ...state,
                workbench_snapshot: mergedSnapshot,
              };
              patchArtworkThumbnails(
                root,
                mergedSnapshot.artwork_items,
                adapter,
              );
              updateThumbnailStatus(root, state.thumbnail_remaining ?? 0);
            }
          } else {
            updateThumbnailStatus(root, prepared.remaining);
          }
          if (prepared.remaining === 0 || prepared.generated === 0) break;
          await new Promise((resolve) => window.setTimeout(resolve, 50));
        }
      } catch (error) {
        state = { ...state, error: errorMessage(error) };
        render();
      } finally {
        thumbnailPreparationRunning = false;
      }
    })();
  };

  render();
  try {
    state = {
      ...(await adapter.startup()),
      active_view: "paintings",
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
      thumbnail_remaining: null,
      pending_selected_artwork_id: null,
      selected_artwork_error: null,
      idea_source_reading: null,
      summary_provider: null,
      summary_notice: null,
      enrichment_progress: null,
      ...clearedCapture(),
    };
    if (state.active_vault && adapter.workbenchSnapshot) {
      state = {
        ...state,
        workbench_snapshot: await adapter.workbenchSnapshot("newest", null, null),
      };
    }
    if (adapter.openAiProviderStatus) {
      state = { ...state, summary_provider: await adapter.openAiProviderStatus() };
    }
    if (state.active_vault && adapter.artworkEnrichmentStatus) {
      state = { ...state, enrichment_progress: await adapter.artworkEnrichmentStatus() };
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
      pending_selected_artwork_id: null,
      selected_artwork_error: null,
    };
    void refreshWorkbench(
      adapter,
      stateForRefresh,
      update,
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
  update: StateUpdate,
  updateSelection: StateUpdate,
  artworkSelection: ArtworkSelection,
  requestEpoch: { value: number },
): void {
  root.querySelectorAll<HTMLButtonElement>("[data-vault-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const purpose = button.dataset.vaultAction as FolderPurpose;
      await chooseVault(purpose, adapter, state, update);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-archive-view]").forEach((button) => {
    button.addEventListener("click", async () => {
      const activeView = button.dataset.archiveView as AppState["active_view"];
      if (activeView !== "paintings" && activeView !== "idea-sources") return;
      requestEpoch.value += 1;
      artworkSelection.cancel();
      await update({
        ...state,
        active_view: activeView,
        workbench_snapshot: state.workbench_snapshot
          ? { ...state.workbench_snapshot, selected_item: null }
          : null,
        pending_selected_artwork_id: null,
        selected_artwork_error: null,
        idea_source_reading: null,
        summary_notice: null,
        ...clearedCapture(),
      });
    });
  });

  root.querySelector<HTMLButtonElement>("[data-settings-view]")?.addEventListener("click", async () => {
    requestEpoch.value += 1;
    artworkSelection.cancel();
    await update({
      ...state,
      active_view: "settings",
      workbench_snapshot: state.workbench_snapshot
        ? { ...state.workbench_snapshot, selected_item: null }
        : null,
      pending_selected_artwork_id: null,
      selected_artwork_error: null,
      idea_source_reading: null,
      summary_notice: null,
      ...clearedCapture(),
    });
  });

  root.querySelector<HTMLButtonElement>("[data-open-link-capture]")?.addEventListener("click", async () => {
    await update({ ...state, ...clearedCapture(), capture_open: true, error: null });
  });
  root.querySelector<HTMLButtonElement>("[data-close-link-capture]")?.addEventListener("click", async () => {
    requestEpoch.value += 1;
    await update({ ...state, ...clearedCapture() });
  });
  root.querySelector<HTMLTextAreaElement>('[name="copied_text"]')?.addEventListener("paste", async (event) => {
    const image = Array.from(event.clipboardData?.items ?? [])
      .find((item) => item.kind === "file" && item.type.startsWith("image/"))
      ?.getAsFile();
    if (!image) return;
    event.preventDefault();
    const capturePastedImage = {
      fileName: image.name || `pasted-image.${image.type.split("/")[1] || "png"}`,
      bytes: Array.from(new Uint8Array(await image.arrayBuffer())),
    };
    const fallback = state.capture_fallback;
    const title = inputValue(root, '[name="capture_title"]') ?? "";
    const savingReason = inputValue(root, '[name="capture_saving_reason"]');
    await update({
      ...state,
      capture_pasted_image: capturePastedImage,
      capture_copied_text: root.querySelector<HTMLTextAreaElement>('[name="copied_text"]')?.value ?? "",
      capture_title: title,
      capture_saving_reason: savingReason ?? "",
      capture_fallback: fallback ? {
        ...fallback,
        title,
        saving_reason: savingReason,
      } : null,
    });
  });
  root.querySelector<HTMLInputElement>('[name="source_link"]:not([readonly])')?.addEventListener("paste", (event) => {
    const text = event.clipboardData?.getData("text/plain")?.trim() ?? "";
    if (!/^https:\/\//i.test(text)) return;
    event.preventDefault();
    const input = event.currentTarget as HTMLInputElement;
    input.value = text;
    if (input.form?.matches("[data-idea-source-capture]")) return;
    input.form?.requestSubmit();
  });
  root.querySelector<HTMLFormElement>("[data-source-link-capture]")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!adapter.captureSourceLink) return;
    const sourceLink = inputValue(root, '[name="source_link"]');
    if (!sourceLink) return;
    const title = inputValue(root, '[name="capture_title"]') ?? "";
    const savingReason = inputValue(root, '[name="capture_saving_reason"]');
    const request = ++requestEpoch.value;
    try {
      await update({
        ...state,
        busy: true,
        error: null,
        capture_source_link: sourceLink,
        capture_title: title,
        capture_saving_reason: savingReason ?? "",
      });
      const result = await adapter.captureSourceLink({ sourceLink, title, savingReason });
      if (request !== requestEpoch.value) return;
      if (result.status === "needs_manual_fallback") {
        await update({
          ...state,
          busy: false,
          capture_open: true,
          capture_fallback: result,
          capture_source_link: result.source_link,
          capture_title: derivedCaptureTitle(result.source_link, result.title),
          capture_saving_reason: result.saving_reason ?? "",
        });
        return;
      }
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, null, state.search_query)
        : state.workbench_snapshot;
      await update({
        ...state,
        busy: false,
        ...clearedCapture(),
        workbench_snapshot: snapshot,
      });
    } catch (error) {
      if (request !== requestEpoch.value) return;
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });
  root.querySelector<HTMLFormElement>("[data-idea-source-capture]")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!adapter.captureIdeaSource) return;
    const sourceLink = inputValue(root, '[name="source_link"]');
    if (!sourceLink) return;
    const title = inputValue(root, '[name="capture_title"]') ?? "";
    const savingReason = inputValue(root, '[name="capture_saving_reason"]');
    const copiedText = root.querySelector<HTMLTextAreaElement>('[name="copied_text"]')?.value.trim() || null;
    const request = ++requestEpoch.value;
    try {
      await update({
        ...state,
        busy: true,
        error: null,
        capture_source_link: sourceLink,
        capture_title: title,
        capture_saving_reason: savingReason ?? "",
        capture_copied_text: copiedText ?? "",
        summary_notice: null,
      });
      const result = await adapter.captureIdeaSource({ sourceLink, title, savingReason, copiedText });
      if (request !== requestEpoch.value) return;
      if (result.status === "needs_manual_fallback") {
        await update({
          ...state,
          busy: false,
          capture_open: true,
          capture_fallback: result,
          capture_source_link: result.source_link,
          capture_title: derivedCaptureTitle(result.source_link, result.title),
          capture_saving_reason: result.saving_reason ?? "",
          capture_copied_text: copiedText ?? "",
          error: null,
        });
        return;
      }
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, result.item.id, state.search_query)
        : state.workbench_snapshot;
      const reading = adapter.readIdeaSource
        ? await adapter.readIdeaSource(result.item.id)
        : null;
      await update({
        ...state,
        busy: false,
        ...clearedCapture(),
        workbench_snapshot: snapshot,
        idea_source_reading: reading,
        summary_notice: captureSummaryNotice(result.summary_status),
      });
    } catch (error) {
      if (request !== requestEpoch.value) return;
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });
  root.querySelector<HTMLFormElement>("[data-manual-fallback-capture]")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const fallbackCapture = state.active_view === "paintings"
      ? adapter.captureArtworkFallback
      : adapter.captureManualFallback;
    if (!fallbackCapture || !state.capture_fallback) return;
    const title = inputValue(root, '[name="capture_title"]') ?? "Untitled Capture";
    const savingReason = inputValue(root, '[name="capture_saving_reason"]');
    const copiedText = state.active_view === "paintings" ? "" : (root.querySelector<HTMLTextAreaElement>('[name="copied_text"]')?.value ?? "");
    if ((!copiedText.trim() && !state.capture_pasted_image) || (state.active_view === "paintings" && !state.capture_pasted_image)) {
      await update({
        ...state,
        capture_title: title,
        capture_saving_reason: savingReason ?? "",
        capture_copied_text: copiedText,
        error: state.active_view === "paintings" ? "Paste an image before saving this artwork fallback." : "Paste source text or an image before saving this fallback.",
      });
      return;
    }
    const request = ++requestEpoch.value;
    try {
      await update({
        ...state,
        busy: true,
        error: null,
        capture_title: title,
        capture_saving_reason: savingReason ?? "",
        capture_copied_text: copiedText,
        capture_fallback: {
          ...state.capture_fallback,
          title,
          saving_reason: savingReason,
        },
      });
      await fallbackCapture({
        sourceLink: state.capture_fallback.source_link,
        title,
        savingReason,
        copiedText: copiedText.trim() || null,
        copiedImage: state.capture_pasted_image,
      });
      if (request !== requestEpoch.value) return;
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, null, state.search_query)
        : state.workbench_snapshot;
      await update({
        ...state,
        busy: false,
        ...clearedCapture(),
        workbench_snapshot: snapshot,
      });
    } catch (error) {
      if (request !== requestEpoch.value) return;
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
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
      const isIdeaSource = state.workbench_snapshot?.idea_sources.some((item) => item.id === itemId) ?? false;
      if (isIdeaSource && adapter.workbenchSnapshot) {
        const request = ++requestEpoch.value;
        try {
          await update({
            ...state,
            active_view: "idea-sources",
            busy: true,
            error: null,
            workbench_snapshot: state.workbench_snapshot
              ? { ...state.workbench_snapshot, selected_item: null }
              : null,
            idea_source_reading: null,
            summary_notice: null,
          });
          const snapshot = await adapter.workbenchSnapshot(state.artwork_sort, itemId, state.search_query);
          const reading = adapter.readIdeaSource ? await adapter.readIdeaSource(itemId) : null;
          if (request !== requestEpoch.value) return;
          await update({
            ...state,
            active_view: "idea-sources",
            busy: false,
            workbench_snapshot: snapshot,
            idea_source_reading: reading,
            error: null,
          });
        } catch (error) {
          if (request !== requestEpoch.value) return;
          await update({ ...state, active_view: "idea-sources", busy: false, error: errorMessage(error) });
        }
        return;
      }
      await refreshWorkbench(adapter, state, update, state.artwork_sort, itemId);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-artwork-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.artworkId;
      if (!itemId) return;
      await artworkSelection.select(itemId);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-idea-source-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.ideaSourceId;
      if (!itemId || !adapter.workbenchSnapshot) return;
      const request = ++requestEpoch.value;
      try {
        await update({ ...state, busy: true, error: null, idea_source_reading: null, summary_notice: null });
        const snapshot = await adapter.workbenchSnapshot(state.artwork_sort, itemId, state.search_query);
        const reading = adapter.readIdeaSource ? await adapter.readIdeaSource(itemId) : null;
        if (request !== requestEpoch.value) return;
        await update({ ...state, busy: false, workbench_snapshot: snapshot, idea_source_reading: reading, error: null });
      } catch (error) {
        if (request !== requestEpoch.value) return;
        await update({ ...state, busy: false, error: errorMessage(error) });
      }
    });
  });

  root.querySelector<HTMLButtonElement>("[data-summarize-idea-source]")?.addEventListener("click", async () => {
    const selected = state.workbench_snapshot?.selected_item;
    if (!selected || !adapter.summarizeIdeaSource) return;
    const request = ++requestEpoch.value;
    try {
      await update({ ...state, busy: true, error: null, summary_notice: "Creating summary…" });
      const result = await adapter.summarizeIdeaSource(selected.id, "standard");
      const snapshot = adapter.workbenchSnapshot
        ? await adapter.workbenchSnapshot(state.artwork_sort, selected.id, state.search_query)
        : state.workbench_snapshot;
      const reading = adapter.readIdeaSource
        ? await adapter.readIdeaSource(selected.id)
        : state.idea_source_reading;
      if (request !== requestEpoch.value) return;
      await update({
        ...state,
        busy: false,
        workbench_snapshot: snapshot,
        idea_source_reading: reading,
        summary_notice: summaryResultNotice(result),
      });
    } catch (error) {
      if (request !== requestEpoch.value) return;
      await update({ ...state, busy: false, summary_notice: null, error: errorMessage(error) });
    }
  });

  root.querySelector<HTMLFormElement>("[data-summary-provider-form]")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!adapter.configureOpenAiProvider || !adapter.openAiProviderStatus) return;
    const apiKey = root.querySelector<HTMLInputElement>('[name="openai_api_key"]')?.value.trim() ?? "";
    const model = inputValue(root, '[name="openai_model"]');
    if (!apiKey) return;
    try {
      await update({ ...state, busy: true, error: null, summary_notice: null });
      await adapter.configureOpenAiProvider({ apiKey, model: model ?? "gpt-4.1-mini" });
      const provider = await adapter.openAiProviderStatus();
      await update({
        ...state,
        busy: false,
        summary_provider: provider,
        summary_notice: provider.configured ? "Summary provider configured for this computer." : null,
      });
    } catch (error) {
      await update({ ...state, busy: false, error: errorMessage(error) });
    }
  });

  root.querySelector<HTMLButtonElement>("[data-close-settings]")?.addEventListener("click", async () => {
    await update({ ...state, active_view: "paintings" });
  });
  root.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-enrichment-budget], [data-enrichment-max-items], [data-enrichment-max-requests], [data-enrichment-max-duration]").forEach((control) => {
    control.addEventListener("change", () => window.localStorage.setItem(`gg-enrichment-${control.dataset.enrichmentBudget !== undefined ? "budget" : control.dataset.enrichmentMaxItems !== undefined ? "max-items" : control.dataset.enrichmentMaxRequests !== undefined ? "max-requests" : "max-duration"}`, control.value));
  });

  root.querySelector<HTMLButtonElement>("[data-close-item-details]")?.addEventListener("click", async () => {
    if (!state.workbench_snapshot) return;
    const selected = state.workbench_snapshot.selected_item;
    const form = root.querySelector<HTMLFormElement>("[data-item-record-form]");
    if (
      selected &&
      form &&
      itemRecordEditIsDirty(itemRecordEdit(form, selected, false), selected) &&
      !window.confirm("Discard unsaved Item Record changes?")
    ) return;
    artworkSelection.cancel();
    await updateSelection({
      ...state,
      workbench_snapshot: { ...state.workbench_snapshot, selected_item: null },
      selected_review_reason_id: null,
      pending_item_edit: null,
      item_record_conflict: null,
      pending_selected_artwork_id: null,
      selected_artwork_error: null,
    });
  });

  root.querySelectorAll<HTMLButtonElement>("[data-review-item-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const itemId = button.dataset.reviewItemId;
      const reasonId = button.dataset.reviewReasonId;
      if (!itemId || !reasonId) return;
      const isIdeaSource = state.workbench_snapshot?.idea_sources.some((item) => item.id === itemId) ?? false;
      if (isIdeaSource && adapter.workbenchSnapshot) {
        const request = ++requestEpoch.value;
        try {
          await update({
            ...state,
            active_view: "idea-sources",
            selected_review_reason_id: reasonId,
            busy: true,
            error: null,
            idea_source_reading: null,
          });
          const snapshot = await adapter.workbenchSnapshot(state.artwork_sort, itemId, state.search_query);
          const reading = adapter.readIdeaSource ? await adapter.readIdeaSource(itemId) : null;
          if (request !== requestEpoch.value) return;
          await update({ ...state, active_view: "idea-sources", busy: false, workbench_snapshot: snapshot, idea_source_reading: reading, error: null });
        } catch (error) {
          if (request !== requestEpoch.value) return;
          await update({ ...state, active_view: "idea-sources", busy: false, error: errorMessage(error) });
        }
        return;
      }
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
    if (adapter.startArtworkEnrichment) {
      const total = state.workbench_snapshot?.artwork_items.length ?? 0;
      const maxItems = Math.max(1, Number(enrichmentSetting("max-items", "25")) || 25);
      const maxRequests = Math.max(1, Number(enrichmentSetting("max-requests", String(maxItems))) || maxItems);
      const maxDurationSeconds = Math.max(10, Number(enrichmentSetting("max-duration", "900")) || 900);
      const budgetMode = enrichmentSetting("budget", "standard") as "off" | "cheap" | "standard" | "deep";
      await update({ ...state, busy: true, error: null, enrichment_progress: {
        runId: "pending", processed: 0, total: Math.min(total, maxItems), enriched: 0, failed: 0, skipped: 0,
        status: "running", currentItemTitle: null,
      }});
      try {
        const progress = await adapter.startArtworkEnrichment({ budgetMode, maxItems, maxRequests, maxDurationSeconds, rerunCompleted: false }, async (next) => {
          await update(current => ({ ...current, busy: true, error: null, enrichment_progress: next }));
        });
        const snapshot = adapter.workbenchSnapshot
          ? await adapter.workbenchSnapshot(state.artwork_sort, state.workbench_snapshot?.selected_item?.id ?? null, state.search_query)
          : state.workbench_snapshot;
        await update(current => ({ ...current, busy: false, workbench_snapshot: snapshot, enrichment_progress: progress, error: null }));
      } catch (error) { const savedProgress = await adapter.artworkEnrichmentStatus?.().catch(() => null);
        await update(current => ({ ...current, busy: false, enrichment_progress: savedProgress ?? (current.enrichment_progress?.runId !== "pending" && current.enrichment_progress ? { ...current.enrichment_progress, status: "paused" } : null), error: errorMessage(error) })); }
      return;
    }
    await refreshWorkbench(
      adapter,
      { ...state, pending_item_edit: null, item_record_conflict: null },
      update,
      state.artwork_sort,
      state.workbench_snapshot?.selected_item?.id ?? null,
      true,
    );
  });

  root.querySelector<HTMLButtonElement>("[data-cancel-enrichment]")?.addEventListener("click", async () => {
    try { await adapter.cancelArtworkEnrichment?.(state.enrichment_progress?.runId); }
    catch (error) { await update(current => ({ ...current, error: errorMessage(error) })); }
  });
  root.querySelector<HTMLButtonElement>("[data-resume-enrichment]")?.addEventListener("click", async () => {
    const runId = state.enrichment_progress?.runId;
    if (!runId || !adapter.resumeArtworkEnrichment) return;
    await update({ ...state, busy: true, error: null });
    try {
      const progress = await adapter.resumeArtworkEnrichment(runId, async (next) => {
        await update(current => ({ ...current, busy: true, error: null, enrichment_progress: next }));
      });
        const snapshot = adapter.workbenchSnapshot
          ? await adapter.workbenchSnapshot(state.artwork_sort, state.workbench_snapshot?.selected_item?.id ?? null, state.search_query)
          : state.workbench_snapshot;
        await update(current => ({ ...current, busy: false, enrichment_progress: progress, workbench_snapshot: snapshot }));
    } catch (error) { const savedProgress = await adapter.artworkEnrichmentStatus?.().catch(() => null);
        await update(current => ({ ...current, busy: false, enrichment_progress: savedProgress ?? (current.enrichment_progress?.runId !== "pending" && current.enrichment_progress ? { ...current.enrichment_progress, status: "paused" } : null), error: errorMessage(error) })); }
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
    ...clearedCapture(),
    active_view: "paintings",
    active_vault: activeVault,
    known_vaults: knownVaults,
    repair_proposal: null,
    notice: null,
    busy: false,
    error: null,
    idea_source_reading: null,
    summary_notice: null,
    enrichment_progress: null,
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
  if (adapter.openAiProviderStatus) {
    nextState = {
      ...nextState,
      summary_provider: await adapter.openAiProviderStatus(),
    };
  }
  if (adapter.artworkEnrichmentStatus) {
    nextState.enrichment_progress = await adapter.artworkEnrichmentStatus();
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
            ${state.active_vault && state.workbench_snapshot ? archiveNavigationTemplate(state) : ""}
          </nav>
      <button type="button" class="settings-navigation-item ${state.active_view === "settings" ? "is-current" : ""}" data-settings-view ${state.busy ? "disabled" : ""} aria-current="${state.active_view === "settings" ? "page" : "false"}">
            <span><strong>Settings</strong><small>Provider & limits</small></span>
          </button>
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
            state.active_view === "settings"
                ? settingsTemplate(state, adapter)
                : state.repair_proposal
                  ? repairVaultTemplate(state)
                  : state.active_vault && state.workbench_snapshot
                ? state.active_view === "idea-sources"
                  ? ideaSourcesWorkbenchTemplate(state, adapter)
                  : artworkWorkbenchTemplate(state, adapter)
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
  const selectedId = state.pending_selected_artwork_id ?? snapshot.selected_item?.id ?? null;

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
          <button class="secondary-button" type="button" data-open-link-capture ${state.busy || !adapter.captureSourceLink ? "disabled" : ""}>
            Capture Link
          </button>
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

      ${adapter.startArtworkEnrichment ? `<p class="form-hint">Refresh researches and tags all paintings in resumable batches, up to ${escapeHtml(enrichmentSetting("max-items", "25"))} paintings and ${escapeHtml(enrichmentSetting("max-requests", "25"))} requests per run. Uses your configured AI provider; monetary cost is unavailable. Change limits in Settings.</p>` : ""}
      ${enrichmentTemplate(state, adapter)}

      ${captureLinkTemplate(state, adapter)}
      ${importRunTemplate(state, adapter)}
      <p class="thumbnail-status" role="status" aria-live="polite" data-thumbnail-status ${state.thumbnail_remaining && state.thumbnail_remaining > 0 ? "" : "hidden"}>${thumbnailStatusText(state.thumbnail_remaining)}</p>

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

      <div class="artwork-content ${selectedId ? "has-selection" : ""}">
        <div class="artwork-gallery" aria-label="Artwork gallery">
          ${
            snapshot.artwork_items.length === 0
              ? '<p class="gallery-empty">No artwork saved yet. Add one or more image files to begin.</p>'
              : snapshot.artwork_items
                  .map(
                    (item) => `
                      <button class="artwork-card ${selectedId === item.id ? "is-selected" : ""}" type="button"
                        data-artwork-id="${escapeHtml(item.id)}" ${state.busy ? "disabled" : ""}>
                        <span class="artwork-preview ${item.thumbnail_is_placeholder ? "is-placeholder" : ""}" data-artwork-preview>
                          <img src="${fileUrl(item.thumbnail_file)}" alt="${escapeHtml(item.title)}" width="480" height="360" loading="lazy" decoding="async">
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

        ${artworkDetailsTemplate(state, adapter)}
      </div>

      ${vaultTrashTemplate(snapshot, state, adapter)}
    </section>
  `;
}

function archiveNavigationTemplate(state: AppState): string {
  const snapshot = state.workbench_snapshot;
  if (!snapshot) return "";
  return `
    <section class="archive-navigation" aria-label="Saved Item areas">
      <p>Browse</p>
      <button type="button" data-archive-view="paintings" class="archive-navigation-item ${state.active_view === "paintings" ? "is-current" : ""}" ${state.busy ? "disabled" : ""} aria-current="${state.active_view === "paintings" ? "page" : "false"}">
        <i data-lucide="images"></i><span><strong>Paintings</strong><small>${snapshot.artwork_items.length}</small></span>
      </button>
      <button type="button" data-archive-view="idea-sources" class="archive-navigation-item ${state.active_view === "idea-sources" ? "is-current" : ""}" ${state.busy ? "disabled" : ""} aria-current="${state.active_view === "idea-sources" ? "page" : "false"}">
        <i data-lucide="book-open"></i><span><strong>Idea Sources</strong><small>${snapshot.idea_sources.length}</small></span>
      </button>
    </section>`;
}

function ideaSourcesWorkbenchTemplate(state: AppState, adapter: DesktopAdapter): string {
  const snapshot = state.workbench_snapshot;
  if (!snapshot) return "";
  const selected = snapshot.selected_item;
  const selectedIdea = selected && snapshot.idea_sources.some((item) => item.id === selected.id)
    ? selected
    : null;
  return `
    <section class="artwork-workspace idea-sources-workspace">
      <header class="artwork-toolbar idea-sources-toolbar">
        <div>
          <p class="eyebrow">Readable source archive</p>
          <h2>Idea Sources</h2>
          <p class="section-intro">Keep the source material and its summary in your Vault, so the idea survives after the tab or page is gone.</p>
        </div>
        <div class="idea-source-actions">
          <button class="primary-button" type="button" data-open-link-capture ${state.busy || !adapter.captureIdeaSource ? "disabled" : ""}>Add Idea Source</button>
        </div>
      </header>

      ${captureLinkTemplate(state, adapter, true)}
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

      <div class="idea-source-browser ${selectedIdea ? "has-selection" : ""}">
        ${ideaSourcesTemplate(snapshot, state)}
        ${selectedIdea ? ideaSourceDetailsTemplate(state, selectedIdea, adapter) : ""}
      </div>
      ${vaultTrashTemplate(snapshot, state, adapter)}
    </section>`;
}

function settingsTemplate(state: AppState, adapter: DesktopAdapter): string {
  const setting = enrichmentSetting;
  return `<section class="settings-workspace" aria-label="Settings">
    <header class="settings-heading"><div><p class="eyebrow">App preferences</p><h2>Settings</h2><p>Provider credentials and bounded enrichment limits apply across the app, even when no Vault is open.</p></div><button class="secondary-button" type="button" data-close-settings>Back to archive</button></header>
    ${summaryProviderTemplate(state, adapter)}
    <section class="settings-card" aria-labelledby="enrichment-settings-title"><h3 id="enrichment-settings-title">Artwork enrichment limits</h3><p>Runs send artwork previews and metadata to your configured provider for tagging and web research. Item, request, and time limits bound each run. Monetary cost is unavailable for the configured model; these are not dollar limits.</p><label>AI budget mode<select data-enrichment-budget><option value="cheap" ${setting("budget", "standard") === "cheap" ? "selected" : ""}>Cheap</option><option value="standard" ${setting("budget", "standard") === "standard" ? "selected" : ""}>Standard</option><option value="deep" ${setting("budget", "standard") === "deep" ? "selected" : ""}>Deep</option></select></label><label>Maximum paintings per run<input type="number" min="1" max="250" value="${escapeHtml(setting("max-items", "25"))}" data-enrichment-max-items></label><label>Maximum requests per run<input type="number" min="1" max="250" value="${escapeHtml(setting("max-requests", "25"))}" data-enrichment-max-requests></label><label>Maximum duration (seconds)<input type="number" min="10" max="3600" value="${escapeHtml(setting("max-duration", "900"))}" data-enrichment-max-duration></label></section>
  </section>`;
}

function enrichmentSetting(key: string, fallback: string): string {
  try { return window.localStorage.getItem(`gg-enrichment-${key}`) ?? fallback; }
  catch { return fallback; }
}

function enrichmentTemplate(state: AppState, adapter: DesktopAdapter): string {
  const progress = state.enrichment_progress;
  if (!progress) return "";
  const percent = progress.total ? Math.round((progress.processed / progress.total) * 100) : 0;
  return `<section class="enrichment-progress" aria-label="Artwork enrichment progress" role="status"><div><strong>Artwork enrichment ${progress.status}</strong><span>${progress.processed} of ${progress.total} · ${percent}% · ${progress.enriched} enriched · ${progress.failed} failed</span></div>${progress.currentItemTitle ? `<p>Current: ${escapeHtml(progress.currentItemTitle)}</p>` : ""}${progress.failures?.length ? `<details><summary>See ${progress.failed} failures</summary><ul>${progress.failures.slice(0, 20).map(failure => `<li>${escapeHtml(failure.title)}: ${escapeHtml(failure.reason)}</li>`).join("")}</ul><p>Refresh Item Records retries unsuccessful paintings.</p></details>` : ""}${progress.status === "running" ? `<button class="secondary-button" type="button" data-cancel-enrichment ${adapter.cancelArtworkEnrichment ? "" : "disabled"}>Cancel</button>` : progress.status === "cancelled" || progress.status === "paused" ? `<button class="secondary-button" type="button" data-resume-enrichment ${adapter.resumeArtworkEnrichment ? "" : "disabled"}>Resume</button>` : ""}</section>`;
}

function summaryProviderTemplate(state: AppState, adapter: DesktopAdapter): string {
  if (!adapter.configureOpenAiProvider) return "";
  const provider = state.summary_provider;
  return `
    <details class="summary-provider">
      <summary>${provider?.configured ? `Summaries: ${escapeHtml(provider.model ?? "configured")}` : "Set up summaries"}</summary>
      <form data-summary-provider-form>
        <p>The API key is stored for this app on this computer, outside the Vault. When you create a summary, the saved source text is sent to OpenAI. Refresh Item Records sends artwork previews and metadata for tagging and research.</p>
        <label>OpenAI API Key<input name="openai_api_key" type="password" autocomplete="off" required></label>
        <label>Model<input name="openai_model" value="${escapeHtml(provider?.model ?? "gpt-4.1-mini")}" required></label>
        <button class="secondary-button" type="submit" ${state.busy ? "disabled" : ""}>Save Summary Settings</button>
      </form>
    </details>`;
}

function captureLinkTemplate(state: AppState, adapter: DesktopAdapter, ideaCapture = false): string {
  if (!state.capture_open) return "";
  const fallback = state.capture_fallback;
  if (ideaCapture) {
    return `
      <section class="capture-panel" aria-label="Add Idea Source">
        <div class="capture-heading">
          <div><p class="eyebrow">Close the tab with confidence</p><h3>${fallback ? "Paste the source text" : "Add an Idea Source"}</h3></div>
          <button class="detail-close" type="button" data-close-link-capture aria-label="Close Add Idea Source"><span aria-hidden="true">×</span></button>
        </div>
        ${fallback ? `<p class="capture-fallback-reason" role="status">${escapeHtml(fallback.reason)} Paste the readable post or page text below; the draft has been kept.</p>` : ""}
        <form class="capture-form idea-capture-form" data-idea-source-capture>
          <label class="capture-span">Source Link<input name="source_link" type="url" placeholder="https://…" value="${escapeHtml(fallback?.source_link ?? state.capture_source_link)}" required></label>
          <label>Title <span>(optional)</span><input name="capture_title" placeholder="Use the page title when available" value="${escapeHtml(fallback?.title ?? state.capture_title)}"></label>
          <label>Saving Reason <span>(optional)</span><input name="capture_saving_reason" placeholder="How might you use this later?" value="${escapeHtml(fallback?.saving_reason ?? state.capture_saving_reason)}"></label>
          <label class="capture-span">Source Text <span>(${fallback ? "required to preserve this source" : "optional fallback"})</span><textarea name="copied_text" rows="8" placeholder="Paste the article or post text here when the page cannot be read automatically" ${fallback ? "required" : ""}>${escapeHtml(state.capture_copied_text)}</textarea></label>
          <p class="form-hint">The readable source is saved locally before summarization. Automatic summaries are attempted only when a provider is configured.</p>
          <button class="primary-button" type="submit" ${state.busy || !adapter.captureIdeaSource ? "disabled" : ""}>${fallback ? "Save Source Text" : "Save Idea Source"}</button>
        </form>
      </section>`;
  }
  return `
    <section class="capture-panel" aria-label="Capture Link">
      <div class="capture-heading">
        <div><p class="eyebrow">Source capture</p><h3>${fallback ? "Manual Fallback" : "Capture a Source Link"}</h3></div>
        <button class="detail-close" type="button" data-close-link-capture aria-label="Close Capture Link"><span aria-hidden="true">×</span></button>
      </div>
      ${fallback ? `
        <p class="capture-fallback-reason" role="status">${escapeHtml(fallback.reason)}</p>
        <form class="capture-form" data-manual-fallback-capture>
          <label>Source Link<input name="source_link" type="url" value="${escapeHtml(fallback.source_link)}" readonly></label>
          <label>Title <span>(optional)</span><input name="capture_title" value="${escapeHtml(fallback.title)}"></label>
          <label>Saving Reason<input name="capture_saving_reason" value="${escapeHtml(fallback.saving_reason ?? "")}"></label>
          ${ideaCapture ? '<label class="capture-span">Copied Text<textarea name="copied_text" rows="5" placeholder="Paste the post or page text without surrounding discussion">' + escapeHtml(state.capture_copied_text) + '</textarea></label>' : '<label class="capture-span">Artwork Image<textarea name="copied_text" rows="3" placeholder="Paste an image here" aria-label="Paste Artwork Image"></textarea></label><p class="form-hint capture-span">Text is not accepted for Paintings.</p>'}
          <p class="form-hint">${ideaCapture ? "Paste an image into the Copied Text field to preserve its bytes too." : "The pasted image will be saved as the artwork’s preserved file."} <span data-pasted-image-status>${state.capture_pasted_image ? `${escapeHtml(state.capture_pasted_image.fileName)} ready to preserve` : ""}</span></p>
          <button class="primary-button" type="submit" ${state.busy || !(ideaCapture ? adapter.captureManualFallback : adapter.captureArtworkFallback) ? "disabled" : ""}>${ideaCapture ? "Save Manual Fallback" : "Save Artwork"}</button>
        </form>
      ` : `
        <form class="capture-form" data-source-link-capture>
          <label>Source Link<input name="source_link" type="url" placeholder="https://…" value="${escapeHtml(state.capture_source_link)}" required></label>
          <label>Saving Reason <span>(optional)</span><input name="capture_saving_reason" value="${escapeHtml(state.capture_saving_reason)}"></label>
          <p class="form-hint">Paste a public HTTPS link. Wikimedia and public X images keep the Best Available File; other sources save the Source Link as an Idea Source.</p>
          <button class="primary-button" type="submit" ${state.busy ? "disabled" : ""}>Capture</button>
        </form>
      `}
    </section>`;
}

function ideaSourcesTemplate(snapshot: WorkbenchSnapshot, state: AppState): string {
  if (snapshot.idea_sources.length === 0) {
    return '<div class="idea-source-empty"><strong>No Idea Sources yet</strong><p>Add a post, blog, or web page to keep its readable text in this Vault.</p></div>';
  }
  return `
    <section class="idea-sources" aria-label="Saved Idea Sources">
      <div class="idea-source-list">
        ${snapshot.idea_sources.map((item) => `
          <button type="button" class="idea-source-card ${snapshot.selected_item?.id === item.id ? "is-selected" : ""}" data-idea-source-id="${escapeHtml(item.id)}" ${state.busy ? "disabled" : ""}>
            <span class="idea-source-card-heading"><strong>${escapeHtml(item.title)}</strong><small>${escapeHtml(sourceHost(item.source_link))}</small></span>
            <span>${escapeHtml(item.saving_reason ?? "No saving reason yet")}</span>
            <span class="preservation-state ${item.source_copy ? "is-preserved" : "is-incomplete"}">${item.source_copy ? "Readable source preserved" : "Source text still needed"}</span>
          </button>`).join("")}
      </div>
    </section>`;
}

function ideaSourceDetailsTemplate(state: AppState, selected: ItemDetails, adapter: DesktopAdapter): string {
  const reading = state.idea_source_reading?.id === selected.id ? state.idea_source_reading : null;
  const provider = state.summary_provider;
  const canSummarize = provider?.configured === true && Boolean(adapter.summarizeIdeaSource);
  const sourceText = reading?.cleaned_text.trim() ?? "";
  const summary = reading?.summary ?? selected.summary;
  return `
    <aside class="idea-source-details" aria-label="Idea Source details">
      <button class="detail-close" type="button" data-close-item-details aria-label="Close Item Details"><span aria-hidden="true">×</span></button>
      <p class="eyebrow">Idea Source</p>
      <h2>${escapeHtml(selected.title)}</h2>
      ${selected.source_link ? `<a class="source-link" href="${escapeHtml(selected.source_link)}" target="_blank" rel="noreferrer">${escapeHtml(selected.source_link)}</a>` : ""}
      ${selected.saving_reason ? `<section class="saving-reason"><strong>Why you saved it</strong><p>${escapeHtml(selected.saving_reason)}</p></section>` : ""}
      <section class="source-summary" aria-labelledby="idea-summary-title">
        <div class="source-section-heading"><h3 id="idea-summary-title">Summary</h3>${canSummarize ? `<button class="secondary-button" type="button" data-summarize-idea-source ${state.busy ? "disabled" : ""}>${summary ? "Refresh Summary" : "Create Summary"}</button>` : ""}</div>
        ${summary ? `<p>${escapeHtml(summary)}</p>` : `<p class="empty-copy">${provider?.configured === false ? "No summary yet. Automatic summarization is unavailable until a provider is configured." : "No summary has been saved for this source."}</p>`}
        ${state.summary_notice ? `<p class="summary-notice" role="status">${escapeHtml(state.summary_notice)}</p>` : ""}
      </section>
      <section class="source-reading" aria-labelledby="source-reading-title">
        <div class="source-section-heading"><h3 id="source-reading-title">Preserved source</h3>${selected.source_copy ? '<span>Saved locally</span>' : ""}</div>
        ${sourceText
          ? `<div class="source-text">${sourceText.split(/\n{2,}/).map((paragraph) => `<p>${escapeHtml(paragraph)}</p>`).join("")}</div>`
          : selected.source_copy
            ? `<p class="empty-copy">The source copy is saved locally, but this app version cannot display it here yet.</p><p class="file-path">${escapeHtml(selected.source_copy)}</p>`
            : '<p class="empty-copy">This item only has a Source Link. Add the source text before treating it as preserved.</p>'}
      </section>
      ${itemRecordConflictTemplate(state)}
      ${itemRecordEditorTemplate(state, selected, adapter)}
      <button class="danger-button" type="button" data-move-to-trash ${adapter.moveItemToTrash ? "" : "disabled"}>Move to Vault Trash</button>
    </aside>`;
}

function artworkDetailsTemplate(state: AppState, adapter: DesktopAdapter): string {
  const snapshot = state.workbench_snapshot;
  if (!snapshot) return "";
  const selectedId = state.pending_selected_artwork_id ?? snapshot.selected_item?.id ?? null;
  if (!selectedId) return "";
  const selected = snapshot.selected_item?.id === selectedId ? snapshot.selected_item : null;
  const selectedArtwork = snapshot.artwork_items.find((item) => item.id === selectedId);
  if (!selected && !selectedArtwork) return "";
  const fileUrl = (path: string) => escapeHtml(adapter.fileUrl?.(path) ?? path);
  if (!selected) {
    const selectionError = state.selected_artwork_error;
    return `
      <aside class="artwork-details" aria-label="Artwork details" aria-busy="${selectionError ? "false" : "true"}">
        <button class="detail-close" type="button" data-close-item-details aria-label="Close Item Details"><span aria-hidden="true">×</span></button>
        <img class="primary-file" data-artwork-detail-preview src="${fileUrl(selectedArtwork!.thumbnail_file)}" alt="Preview of ${escapeHtml(selectedArtwork!.title)}" width="480" height="360" decoding="async">
        <p class="eyebrow">${selectionError ? "Item Details unavailable" : "Loading Item Record…"}</p>
        <h2>${escapeHtml(selectedArtwork!.title)}</h2>
        <p class="detail-byline">${escapeHtml(metadataLine(selectedArtwork!.creator, selectedArtwork!.year))}</p>
        ${selectionError ? `<p class="error-message" role="alert">${escapeHtml(selectionError)}</p>` : ""}
      </aside>`;
  }
  return `
    <aside class="artwork-details" aria-label="Artwork details">
      <button class="detail-close" type="button" data-close-item-details aria-label="Close Item Details"><span aria-hidden="true">×</span></button>
      ${selectedArtwork ? `<img class="primary-file" data-artwork-detail-preview src="${fileUrl(selectedArtwork.thumbnail_file)}" alt="Preview of ${escapeHtml(selected.title)}" width="480" height="360" decoding="async">` : ""}
      ${selectedArtwork ? '<p class="eyebrow">Thumbnail preview</p>' : ""}
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
    </aside>`;
}

function patchArtworkSelection(
  root: HTMLElement,
  adapter: DesktopAdapter,
  state: AppState,
  update: StateUpdate,
  updateSelection: StateUpdate,
  artworkSelection: ArtworkSelection,
  requestEpoch: { value: number },
): void {
  const content = root.querySelector<HTMLElement>(".artwork-content");
  if (!content || !state.workbench_snapshot) {
    void update(state);
    return;
  }

  const selectedId = state.pending_selected_artwork_id
    ?? state.workbench_snapshot.selected_item?.id
    ?? null;
  content.classList.toggle("has-selection", selectedId !== null);
  content.querySelectorAll<HTMLElement>("[data-artwork-id]").forEach((card) => {
    card.classList.toggle("is-selected", card.dataset.artworkId === selectedId);
  });
  content.querySelector<HTMLElement>(".artwork-details")?.remove();
  if (selectedId) {
    content.insertAdjacentHTML("beforeend", artworkDetailsTemplate(state, adapter));
    const details = content.querySelector<HTMLElement>(".artwork-details");
    if (details) {
      bindActions(details, adapter, state, update, updateSelection, artworkSelection, requestEpoch);
    }
  }
}

function vaultTrashTemplate(snapshot: WorkbenchSnapshot, state: AppState, adapter: DesktopAdapter): string {
  const items = snapshot.trashed_items ?? [];
  if (items.length === 0) {
    return `<section class="vault-trash is-empty" aria-label="Vault Trash"><strong>Vault Trash</strong><span>Vault Trash is empty.</span></section>`;
  }
  return `
    <section class="vault-trash" aria-label="Vault Trash">
      <details class="vault-trash-details">
        <summary><span><small>Recoverable removal</small><strong>Vault Trash</strong></span><span>${items.length} ${items.length === 1 ? "item" : "items"}</span></summary>
        <div class="vault-trash-items">
          ${items.map(item => `<article><h3>${escapeHtml(item.title)}</h3><p>${escapeHtml(metadataLine(item.creator ?? "", item.year ?? ""))} · ${escapeHtml(item.home_subvault)}</p><p>Review: ${escapeHtml(item.review_status ?? "Unknown")} · Tags: ${escapeHtml(item.tags?.join(", ") || "None")}</p><p class="file-path">${escapeHtml(item.item_folder)}</p><p>Collections: ${escapeHtml(item.collections.join(", ") || "None")} (target in Vault Trash)</p><p>Incoming Item Links: ${escapeHtml(item.incoming_item_links.map(link => `${link.label} (${link.source_item_id}) → target in Vault Trash`).join(", ") || "None")}</p><button class="secondary-button" type="button" data-restore-trash-id="${escapeHtml(item.id)}" ${state.busy || !adapter.restoreTrashedItem ? "disabled" : ""}>Restore</button><div><label>Type stable item ID <code>${escapeHtml(item.id)}</code> to permanently delete<input data-delete-confirmation aria-label="Confirm permanent deletion for ${escapeHtml(item.title)}"></label><button class="danger-button" type="button" data-permanently-delete-trash-id="${escapeHtml(item.id)}" ${state.busy || !adapter.permanentlyDeleteTrashedItem ? "disabled" : ""}>Permanently Delete</button></div></article>`).join("")}
        </div>
      </details>
    </section>`;
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
  const visibleItems = snapshot.review_queue.slice(0, 12);
  const reasonCount = snapshot.review_queue.reduce(
    (total, item) => total + item.review_reasons.length,
    0,
  );
  return `
    <section class="review-queue" aria-label="Review Queue">
      <div class="review-queue-heading">
        <p class="eyebrow">Review queue</p>
        <p><strong>${reasonCount} ${reasonCount === 1 ? "concern" : "concerns"}</strong> across ${snapshot.review_queue.length} ${snapshot.review_queue.length === 1 ? "item" : "items"}${visibleItems.length < snapshot.review_queue.length ? ` · showing the first ${visibleItems.length}` : ""}</p>
      </div>
      <div class="review-queue-items">
        ${visibleItems
          .map((item) => {
            const firstReason = item.review_reasons[0];
            if (!firstReason) return "";
            const reasonMessages = item.review_reasons.map((reason) => reason.message).join(" · ");
            return `
              <button type="button" data-review-item-id="${escapeHtml(item.id)}"
                data-review-reason-id="${escapeHtml(firstReason.id)}" aria-label="${escapeHtml(`${item.title} ${reasonMessages}`)}" ${state.busy ? "disabled" : ""}>
                <strong>${escapeHtml(item.title)}</strong>
                <span>${item.review_reasons.length} ${item.review_reasons.length === 1 ? "concern" : "concerns"}</span>
                <small>${escapeHtml(reasonMessages)}</small>
              </button>`;
          })
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

function updateThumbnailStatus(root: HTMLElement, remaining: number): void {
  const status = root.querySelector<HTMLElement>("[data-thumbnail-status]");
  if (!status) return;
  status.hidden = remaining === 0;
  status.textContent = thumbnailStatusText(remaining);
}

function patchArtworkThumbnails(
  root: HTMLElement,
  artworkItems: ArtworkGridItem[],
  adapter: DesktopAdapter,
): void {
  const artworkById = new Map(artworkItems.map((item) => [item.id, item]));
  root.querySelectorAll<HTMLElement>(".artwork-card[data-artwork-id]").forEach((card) => {
    const item = artworkById.get(card.dataset.artworkId ?? "");
    if (!item) return;
    const preview = card.querySelector<HTMLElement>("[data-artwork-preview]");
    preview?.classList.toggle("is-placeholder", item.thumbnail_is_placeholder);
    const image = preview?.querySelector<HTMLImageElement>("img");
    if (image) image.src = adapter.fileUrl?.(item.thumbnail_file) ?? item.thumbnail_file;
  });

  const selectedId = root
    .querySelector<HTMLElement>(".artwork-card.is-selected[data-artwork-id]")
    ?.dataset.artworkId;
  const selected = selectedId ? artworkById.get(selectedId) : null;
  const detailPreview = root.querySelector<HTMLImageElement>("[data-artwork-detail-preview]");
  if (selected && detailPreview) {
    detailPreview.src = adapter.fileUrl?.(selected.thumbnail_file) ?? selected.thumbnail_file;
  }
}

function thumbnailStatusText(remaining: number | null): string {
  return remaining && remaining > 0
    ? `Preparing thumbnail previews in the background — ${remaining} remaining`
    : "";
}

function metadataLine(creator: string, year: string): string {
  return [creator, year].filter(Boolean).join(" · ") || "Unknown creator";
}

function derivedCaptureTitle(sourceLink: string, fallbackTitle: string): string {
  const trimmed = fallbackTitle.trim();
  if (trimmed) return trimmed;
  try {
    const parsed = new URL(sourceLink);
    const host = parsed.hostname.replace(/^www\./, "");
    if (host === "x.com" || host === "twitter.com") {
      const handle = parsed.pathname.split("/").filter(Boolean)[0];
      if (handle) return `X post by ${handle}`;
    }
    if (host === "commons.wikimedia.org") {
      const leaf = decodeURIComponent(parsed.pathname.split("/").pop() ?? "");
      const stem = leaf.replace(/^File:/i, "").replace(/_/g, " ").replace(/\.[^.]+$/, "");
      if (stem) return stem;
    }
  } catch {
    /* keep default title */
  }
  return "Untitled Capture";
}

function sourceHost(sourceLink: string): string {
  try {
    return new URL(sourceLink).hostname.replace(/^www\./, "");
  } catch {
    return "Saved source";
  }
}

function captureSummaryNotice(status: "generated" | "unavailable" | "skipped"): string {
  if (status === "generated") return "Summary created and saved with the source.";
  if (status === "unavailable") return "Source saved; summary unavailable. Check settings or retry.";
  return "Source saved without an automatic summary.";
}

function summaryResultNotice(result: {
  status: "generated" | "unavailable" | "skipped" | "failed";
  reason?: string | null;
}): string {
  if (result.status === "generated") return "Summary created and saved.";
  if (result.reason) return result.reason;
  if (result.status === "unavailable") return "Automatic summarization is not configured.";
  if (result.status === "failed") return "The summary could not be created. The preserved source is unchanged.";
  return "No summary was created. The preserved source is unchanged.";
}

function inputValue(root: HTMLElement, selector: string): string | null {
  const value = root.querySelector<HTMLInputElement>(selector)?.value.trim();
  return value || null;
}

function clearedCapture(): Pick<
  AppState,
  | "capture_open"
  | "capture_fallback"
  | "capture_pasted_image"
  | "capture_copied_text"
  | "capture_source_link"
  | "capture_title"
  | "capture_saving_reason"
> {
  return {
    capture_open: false,
    capture_fallback: null,
    capture_pasted_image: null,
    capture_copied_text: "",
    capture_source_link: "",
    capture_title: "",
    capture_saving_reason: "",
  };
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
