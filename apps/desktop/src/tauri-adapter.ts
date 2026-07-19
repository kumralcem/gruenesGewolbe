import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  ActiveVault,
  ArtworkImportMetadata,
  ArtworkSort,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
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
    workbenchSnapshot: (artworkSort: ArtworkSort, selectedItemId: string | null) =>
      invoke<WorkbenchSnapshot>("workbench_snapshot", { artworkSort, selectedItemId }),
    fileUrl: convertFileSrc,
  };
}
