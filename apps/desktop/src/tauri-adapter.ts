import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

import type {
  ActiveVault,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  ImportProgress,
  ImportRunSummary,
  ItemDetails,
  ItemRecordEdit,
  ItemRecordSaveResult,
  OpenVaultResult,
  SelectedFileImportSummary,
  WorkbenchSnapshot,
} from "./contracts";

export function createTauriAdapter(): DesktopAdapter {
  return {
    startup: () => invoke<DesktopStartup>("startup"),
    selectFolder: (purpose: FolderPurpose) =>
      open({
        directory: true,
        multiple: false,
        title: purpose === "create" ? "Create Vault" : "Open Vault",
      }),
    createVault: (root: string) => invoke<ActiveVault>("create_vault", { root }),
    openVault: (root: string) => invoke<OpenVaultResult>("open_vault", { root }),
    confirmVaultRepair: (root: string) => invoke<ActiveVault>("confirm_vault_repair", { root }),
    cancelVaultRepair: (root: string) => invoke<void>("cancel_vault_repair", { root }),
    selectArtworkFiles: async () => {
      const selected = await open({
        multiple: true,
        title: "Add Artwork",
        filters: [
          { name: "Artwork images", extensions: ["jpg", "jpeg", "png", "webp", "gif"] },
        ],
      });
      if (!selected) return [];
      return Array.isArray(selected) ? selected : [selected];
    },
    addArtworkFiles: (sourceFiles, options) =>
      invoke<SelectedFileImportSummary>("add_artwork_files", {
        sourceFiles,
        creator: options.metadata.creator,
        year: options.metadata.year,
        savingReason: options.metadata.savingReason,
        importExactDuplicates: options.importExactDuplicates,
      }),
    selectImportFolder: () =>
      open({ directory: true, multiple: false, title: "Import Paintings Folder" }),
    runPaintingsImport: async (sourceFolder, options, onProgress) => {
      const unlisten = await listen<ImportProgress>("import-progress", (event) => {
        void onProgress(event.payload);
      });
      try {
        return await invoke<ImportRunSummary>("run_paintings_import", {
          sourceFolder,
          creator: options.metadata.creator,
          year: options.metadata.year,
          savingReason: options.metadata.savingReason,
          importExactDuplicates: options.importExactDuplicates,
        });
      } finally {
        unlisten();
      }
    },
    cancelPaintingsImport: () => invoke<void>("cancel_paintings_import"),
    workbenchSnapshot: (artworkSort, selectedItemId, searchQuery) =>
      invoke<WorkbenchSnapshot>("workbench_snapshot", {
        artworkSort,
        selectedItemId,
        searchQuery,
      }),
    refreshWorkbenchSnapshot: (artworkSort, selectedItemId, searchQuery) =>
      invoke<WorkbenchSnapshot>("refresh_workbench", {
        artworkSort,
        selectedItemId,
        searchQuery,
      }),
    openActivityLog: async () => {
      const path = await invoke<string>("activity_log_path");
      await openPath(path);
    },
    saveItemRecord: (edit: ItemRecordEdit) =>
      invoke<ItemRecordSaveResult>("save_item_record", {
        id: edit.id,
        expectedRevision: edit.expected_revision,
        overwriteConflict: edit.overwrite_conflict,
        title: edit.title,
        creator: edit.creator,
        year: edit.year,
        savingReason: edit.saving_reason,
        summary: edit.summary,
        tags: edit.tags,
      }),
    resolveReviewReason: (resolution) =>
      invoke<ItemDetails>("resolve_review_reason", {
        itemId: resolution.item_id,
        reasonId: resolution.reason_id,
        expectedRevision: resolution.expected_revision,
        action: resolution.action,
        correction: resolution.correction,
      }),
    confirmItemFolderRename: (id, proposal) =>
      invoke<ItemDetails>("confirm_item_folder_rename", {
        id,
        currentPath: proposal.current_path,
        proposedPath: proposal.proposed_path,
      }),
    fileUrl: convertFileSrc,
  };
}
