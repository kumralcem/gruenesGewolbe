export interface ActiveVault {
  root: string;
}

export interface KnownVault {
  root: string;
}

export interface VaultRepairProposal {
  root: string;
  directories: string[];
}

export type OpenVaultResult =
  | { status: "opened"; vault: ActiveVault }
  | { status: "repair_required"; proposal: VaultRepairProposal };

export interface DesktopStartup {
  active_vault: ActiveVault | null;
  known_vaults: KnownVault[];
  repair_proposal: VaultRepairProposal | null;
  notice: string | null;
}

export type FolderPurpose = "create" | "open";

export interface DesktopAdapter {
  startup(): Promise<DesktopStartup>;
  selectFolder(purpose: FolderPurpose): Promise<string | null>;
  createVault(root: string): Promise<ActiveVault>;
  openVault(root: string): Promise<OpenVaultResult>;
  confirmVaultRepair(root: string): Promise<ActiveVault>;
  cancelVaultRepair(root: string): Promise<void>;
}

declare global {
  interface Window {
    __GG_TEST_ADAPTER__?: DesktopAdapter;
  }
}
