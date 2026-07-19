import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  ActiveVault,
  ArtworkImportMetadata,
  ArtworkSort,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  ImportProgress,
  ImportRunSummary,
  OpenVaultResult,
  SavedItem,
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
    addArtworkFiles: (sourceFiles: string[], metadata: ArtworkImportMetadata) =>
      invoke<SavedItem[]>("add_artwork_files", {
        sourceFiles,
        creator: metadata.creator,
        year: metadata.year,
        savingReason: metadata.savingReason,
      }),
    selectImportFolder: () =>
      open({ directory: true, multiple: false, title: "Import Paintings Folder" }),
    runPaintingsImport: async (sourceFolder, onProgress) => {
      const unlisten = await listen<ImportProgress>("import-progress", (event) => {
        void onProgress(event.payload);
      });
      try {
        return await invoke<ImportRunSummary>("run_paintings_import", { sourceFolder });
      } finally {
        unlisten();
      }
    },
    cancelPaintingsImport: () => invoke<void>("cancel_paintings_import"),
    workbenchSnapshot: (artworkSort: ArtworkSort, selectedItemId: string | null) =>
      invoke<WorkbenchSnapshot>("workbench_snapshot", { artworkSort, selectedItemId }),
    fileUrl: convertFileSrc,
  };
}
