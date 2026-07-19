import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  ActiveVault,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
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
    openVault: (root: string) => invoke<ActiveVault>("open_vault", { root }),
  };
}
