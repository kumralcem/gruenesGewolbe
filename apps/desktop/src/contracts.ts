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

export interface SourceLinkCaptureRequest {
  sourceLink: string;
  title: string;
  savingReason: string | null;
}

export interface IdeaSourceCaptureRequest extends SourceLinkCaptureRequest {
  copiedText: string | null;
}

export type IdeaSourceCaptureResult =
  | { status: "captured"; item: SavedItem; summary_status: "generated" | "unavailable" | "skipped"; summary: string | null }
  | {
      status: "needs_manual_fallback";
      source_link: string;
      title: string;
      saving_reason: string | null;
      reason: string;
    };

export interface IdeaSourceContent {
  id: string;
  source_link: string;
  cleaned_text: string;
  summary: string | null;
}

export type AiBudgetMode = "off" | "cheap" | "standard" | "deep";
export type IdeaSummaryResult =
  | { status: "generated"; summary: string }
  | { status: "unavailable" | "skipped" | "failed"; reason?: string | null };
export interface OpenAiProviderStatus { configured: boolean; model: string | null; }
export interface OpenAiProviderConfiguration { apiKey: string; model: string; }

export interface ArtworkEnrichmentOptions {
  budgetMode: AiBudgetMode;
  maxItems: number;
  maxRequests?: number;
  maxDurationSeconds?: number;
  rerunCompleted?: boolean;
}

export interface ArtworkEnrichmentProgress {
  runId: string;
  processed: number;
  total: number;
  enriched: number;
  failed: number;
  skipped: number;
  requestCount?: number;
  remaining?: number;
  failures?: Array<{ itemId: string; title: string; reason: string }>;
  status: "running" | "paused" | "cancelled" | "completed";
  currentItemTitle: string | null;
}

export interface ManualFallbackCaptureRequest extends SourceLinkCaptureRequest {
  copiedText: string | null;
  copiedImage: { fileName: string; bytes: number[] } | null;
}

export type SourceLinkCaptureResult =
  | { status: "captured"; item: SavedItem }
  | {
      status: "needs_manual_fallback";
      source_link: string;
      title: string;
      saving_reason: string | null;
      reason: string;
    };

export interface ArtworkImportMetadata {
  creator: string | null;
  year: string | null;
  savingReason: string | null;
}

export interface ArtworkImportOptions {
  metadata: ArtworkImportMetadata;
  importExactDuplicates: boolean;
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

export interface VaultProblem {
  path: string;
  error: string;
}

export type ImportVaultProblem = VaultProblem;

export interface ArtworkImportOutcome {
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
  vault_problems: ImportVaultProblem[];
}

export type ImportRunSummary = ArtworkImportOutcome;
export type SelectedFileImportSummary = ArtworkImportOutcome;

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
  review_reasons: ReviewReason[];
  duplicate_candidates?: Array<{ item_id: string; signal: string }>;
  tags: string[];
  collections: string[];
  item_links: Array<{ link_type: string; target: string; label: string; target_in_vault_trash?: boolean }>;
  saving_reason: string | null;
  source_link: string | null;
  summary: string | null;
  source_copy: string | null;
  record_revision: string;
  folder_rename_proposal: ItemFolderRenameProposal | null;
}

export interface ReviewReason {
  id: string;
  kind: string;
  target_field: string | null;
  message: string;
  evidence: string;
  candidate_item_id?: string | null;
}

export interface IdeaSourceListItem { id: string; title: string; source_link: string; source_copy: string | null; review_status: string; saving_reason: string | null; }

export interface ReviewQueueItem {
  id: string;
  home_subvault: string;
  item_type: string;
  title: string;
  review_status: string;
  saving_reason: string | null;
  review_reasons: ReviewReason[];
}

export type ReviewReasonAction = "accept" | "correct" | "dismiss";

export interface ReviewReasonResolution {
  item_id: string;
  reason_id: string;
  expected_revision: string;
  action: ReviewReasonAction;
  correction: string | null;
}

export type DuplicateCandidateAction = "not-a-duplicate" | "keep-both" | "move-this-item-to-vault-trash";
export interface DuplicateCandidateResolution { item_id: string; reason_id: string; expected_revision: string; action: DuplicateCandidateAction; }

export interface ItemFolderRenameProposal {
  current_path: string;
  proposed_path: string;
}

export interface ItemRecordEdit {
  id: string;
  expected_revision: string;
  overwrite_conflict: boolean;
  title: string;
  creator: string;
  year: string;
  saving_reason: string;
  summary: string;
  tags: string[];
}

export type ItemRecordSaveResult =
  | { status: "saved"; item: ItemDetails }
  | { status: "conflict"; external_item: ItemDetails };

export interface WorkbenchSnapshot {
  active_vault: ActiveVault;
  subvaults: string[];
  collections: Array<{ id: string; name: string }>;
  artwork_items: ArtworkGridItem[];
  idea_sources: IdeaSourceListItem[];
  review_queue: ReviewQueueItem[];
  search_results: SearchResult[];
  selected_item: ItemDetails | null;
  trashed_items?: TrashedItem[];
  vault_problems: VaultProblem[];
}
export interface ThumbnailPreparation { generated: number; remaining: number; }
export interface TrashedItem { id: string; home_subvault: string; item_folder: string; title: string; creator?: string; year?: string; review_status?: string; tags?: string[]; collections: string[]; incoming_item_links: Array<{ source_item_id: string; label: string }>; }
export interface PermanentDeletion { id: string; collections: string[]; incoming_item_links: Array<{ source_item_id: string; label: string }>; }

export interface SearchResult {
  id: string;
  home_subvault: string;
  title: string;
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
    options: ArtworkImportOptions,
  ): Promise<SelectedFileImportSummary>;
  selectImportFolder?(): Promise<string | null>;
  runPaintingsImport?(
    sourceFolder: string,
    options: ArtworkImportOptions,
    onProgress: (progress: ImportProgress) => void | Promise<void>,
  ): Promise<ImportRunSummary>;
  cancelPaintingsImport?(): Promise<void>;
  workbenchSnapshot?(
    sort: ArtworkSort,
    selectedItemId: string | null,
    searchQuery?: string | null,
  ): Promise<WorkbenchSnapshot>;
  refreshWorkbenchSnapshot?(
    sort: ArtworkSort,
    selectedItemId: string | null,
    searchQuery?: string | null,
  ): Promise<WorkbenchSnapshot>;
  prepareThumbnailPreviews?(limit: number): Promise<ThumbnailPreparation>;
  openActivityLog?(): Promise<void>;
  saveItemRecord?(edit: ItemRecordEdit): Promise<ItemRecordSaveResult>;
  resolveReviewReason?(resolution: ReviewReasonResolution): Promise<ItemDetails>;
  resolveDuplicateCandidate?(resolution: DuplicateCandidateResolution): Promise<{ status: "active" | "moved-to-vault-trash" }>;
  confirmItemFolderRename?(
    id: string,
    proposal: ItemFolderRenameProposal,
  ): Promise<ItemDetails>;
  moveItemToTrash?(id: string): Promise<SavedItem>;
  restoreTrashedItem?(id: string): Promise<SavedItem>;
  permanentlyDeleteTrashedItem?(id: string, confirmedId: string): Promise<PermanentDeletion>;
  getItemDetails?(id: string): Promise<ItemDetails>;
  captureIdeaSource?(request: IdeaSourceCaptureRequest): Promise<IdeaSourceCaptureResult>;
  readIdeaSource?(id: string): Promise<IdeaSourceContent>;
  summarizeIdeaSource?(id: string, budgetMode: AiBudgetMode): Promise<IdeaSummaryResult>;
  configureOpenAiProvider?(configuration: OpenAiProviderConfiguration): Promise<void>;
  openAiProviderStatus?(): Promise<OpenAiProviderStatus>;
  captureSourceLink?(request: SourceLinkCaptureRequest): Promise<SourceLinkCaptureResult>;
  captureManualFallback?(request: ManualFallbackCaptureRequest): Promise<SavedItem>;
  captureArtworkFallback?(request: ManualFallbackCaptureRequest): Promise<SavedItem>;
  startArtworkEnrichment?(
    options: ArtworkEnrichmentOptions,
    onProgress: (progress: ArtworkEnrichmentProgress) => void | Promise<void>,
  ): Promise<ArtworkEnrichmentProgress>;
  cancelArtworkEnrichment?(runId?: string): Promise<void>;
  resumeArtworkEnrichment?(
    runId: string,
    onProgress: (progress: ArtworkEnrichmentProgress) => void | Promise<void>,
  ): Promise<ArtworkEnrichmentProgress>;
  artworkEnrichmentStatus?(): Promise<ArtworkEnrichmentProgress | null>;
  fileUrl?(path: string): string;
}

declare global {
  interface Window {
    __GG_TEST_ADAPTER__?: DesktopAdapter;
  }
}
