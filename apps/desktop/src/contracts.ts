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
export type ArtworkSort = "newest" | "oldest" | "title" | "creator" | "year";

export interface SavedItem {
  id: string;
  home_subvault: string;
  item_folder: string;
}

export interface ArtworkGridItem {
  id: string;
  title: string;
  creator: string;
  year: string;
  primary_file: string;
  thumbnail_file: string;
  thumbnail_is_placeholder: boolean;
  review_status: string;
}

export interface ItemDetails {
  id: string;
  home_subvault: string;
  item_folder: string;
  title: string;
  creator: string;
  year: string;
  primary_file: string;
  review_status: string;
  tags: string[];
  collections: string[];
  saving_reason: string | null;
  source_link: string | null;
  summary: string | null;
  source_copy: string | null;
}

export interface WorkbenchSnapshot {
  active_vault: ActiveVault;
  subvaults: string[];
  collections: Array<{ id: string; name: string }>;
  artwork_items: ArtworkGridItem[];
  idea_sources: unknown[];
  review_queue: unknown[];
  search_results: unknown[];
  selected_item: ItemDetails | null;
}

export interface DesktopAdapter {
  startup(): Promise<DesktopStartup>;
  selectFolder(purpose: FolderPurpose): Promise<string | null>;
  createVault(root: string): Promise<ActiveVault>;
  openVault(root: string): Promise<OpenVaultResult>;
  confirmVaultRepair(root: string): Promise<ActiveVault>;
  cancelVaultRepair(root: string): Promise<void>;
  selectArtworkFiles?(): Promise<string[]>;
  addArtworkFiles?(sourceFiles: string[]): Promise<SavedItem[]>;
  workbenchSnapshot?(
    sort: ArtworkSort,
    selectedItemId: string | null,
  ): Promise<WorkbenchSnapshot>;
  fileUrl?(path: string): string;
}

declare global {
  interface Window {
    __GG_TEST_ADAPTER__?: DesktopAdapter;
  }
}
