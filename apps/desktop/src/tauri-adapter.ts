import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  ActiveVault,
  DesktopAdapter,
  DesktopStartup,
  FolderPurpose,
  OpenVaultResult,
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
  };
}
