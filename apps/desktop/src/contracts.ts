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

export interface ArtworkImportMetadata {
  creator: string | null;
  year: string | null;
  savingReason: string | null;
}

export interface ImportProgress {
  processed: number;
  total: number;
  current_file: string;
}

export interface ImportSkippedEntry {
  path: string;
  reason: string;
  existing_item_id: string | null;
}

export interface ImportDuplicateCandidateEntry {
  path: string;
  item_id: string;
  candidate_count: number;
}

export interface ImportFailedEntry {
  path: string;
  error: string;
}

export interface ImportRunSummary {
  imported_count: number;
  skipped_count: number;
  duplicate_candidate_count: number;
  exact_duplicate_count: number;
  cancelled_count: number;
  failed_count: number;
  cancelled: boolean;
  imported_items: SavedItem[];
  skipped_entries: ImportSkippedEntry[];
  duplicate_candidate_entries: ImportDuplicateCandidateEntry[];
  cancelled_files: string[];
  failed_entries: ImportFailedEntry[];
  maintenance_errors: string[];
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
  addArtworkFiles?(
    sourceFiles: string[],
    metadata: ArtworkImportMetadata,
  ): Promise<SavedItem[]>;
  selectImportFolder?(): Promise<string | null>;
  runPaintingsImport?(
    sourceFolder: string,
    metadata: ArtworkImportMetadata,
    onProgress: (progress: ImportProgress) => void | Promise<void>,
  ): Promise<ImportRunSummary>;
  cancelPaintingsImport?(): Promise<void>;
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
