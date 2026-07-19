export interface ActiveVault {
  root: string;
}

export interface KnownVault {
  root: string;
}

export interface DesktopStartup {
  active_vault: ActiveVault | null;
  known_vaults: KnownVault[];
  notice: string | null;
}

export type FolderPurpose = "create" | "open";

export interface DesktopAdapter {
  startup(): Promise<DesktopStartup>;
  selectFolder(purpose: FolderPurpose): Promise<string | null>;
  createVault(root: string): Promise<ActiveVault>;
  openVault(root: string): Promise<ActiveVault>;
}

declare global {
  interface Window {
    __GG_TEST_ADAPTER__?: DesktopAdapter;
  }
}
