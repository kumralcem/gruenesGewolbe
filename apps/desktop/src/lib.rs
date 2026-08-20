use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

const KNOWN_VAULTS_FILE: &str = "known-vaults.tsv";
const LAST_ACTIVE_VAULT_FILE: &str = "last-active-vault.txt";
const OPENAI_PROVIDER_FILE: &str = "openai-provider.toml";

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

use gruenes_gewolbe_core::{
    AddArtworkItem, AiBudgetMode, AiEnrichmentResult, AiProvider, ArtworkGridItem,
    ArtworkImportMetadata, ArtworkImportOptions, ArtworkImportOutcome, ArtworkSort, Collection,
    CollectionDefinition, ExactDuplicatePolicy, ExtractedTextCapture, IdeaSourceListItem,
    ImportProgress, ImportRunAction, ImportRunSummary, ItemDetails, ItemFolderRenameProposal,
    ItemLinkDefinition, ItemRecordEdit, ManualFallbackCapture, ReviewQueueItem, ReviewReason,
    ReviewReasonAction, ReviewReasonResolution, SavedItem, SearchResult, SelectedFileImportSummary,
    SourceCaptureResult, SourceExtractor, SourceLinkCapture, TagDefinition, UpdateItemRecord,
    Vault, VaultError, VaultOpen, VaultProblem, VaultRepairProposal,
};

#[derive(Debug, Default)]
pub struct DesktopShell {
    active_vault: Option<Vault>,
    pending_vault_repair: Option<VaultRepairProposal>,
    app_state_dir: Option<PathBuf>,
    known_vault_roots: Vec<PathBuf>,
    startup_notice: Option<String>,
}

impl DesktopShell {
    pub fn with_app_state_dir(app_state_dir: impl AsRef<Path>) -> Self {
        let app_state_dir = app_state_dir.as_ref().to_path_buf();
        let known_vault_roots = read_known_vault_roots(&app_state_dir).unwrap_or_default();
        let (active_vault, pending_vault_repair, startup_notice) =
            match read_last_active_vault_root(&app_state_dir) {
                Ok(Some(root)) => match Vault::open_or_repair(&root) {
                    Ok(VaultOpen::Opened(vault)) => (Some(vault), None, None),
                    Ok(VaultOpen::RepairRequired(proposal)) => (None, Some(proposal), None),
                    Err(_) => (
                        None,
                        None,
                        Some(format!(
                            "last active vault is unavailable: {}",
                            root.display()
                        )),
                    ),
                },
                Ok(None) => (None, None, None),
                Err(error) => (
                    None,
                    None,
                    Some(format!("last active vault could not be read: {error}")),
                ),
            };
        Self {
            active_vault,
            pending_vault_repair,
            app_state_dir: Some(app_state_dir),
            known_vault_roots,
            startup_notice,
        }
    }

    pub fn create_vault(&mut self, root: impl AsRef<Path>) -> Result<ActiveVault, VaultError> {
        let vault = Vault::create(root)?;
        self.set_active_vault(vault)
    }

    pub fn open_vault(&mut self, root: impl AsRef<Path>) -> Result<ActiveVault, VaultError> {
        let vault = Vault::open(root)?;
        self.set_active_vault(vault)
    }

    pub fn request_open_vault(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<OpenVaultResult, VaultError> {
        match Vault::open_or_repair(root)? {
            VaultOpen::Opened(vault) => self.set_active_vault(vault).map(OpenVaultResult::Opened),
            VaultOpen::RepairRequired(proposal) => {
                self.pending_vault_repair = Some(proposal.clone());
                Ok(OpenVaultResult::RepairRequired(proposal))
            }
        }
    }

    pub fn cancel_vault_repair(&mut self, root: impl AsRef<Path>) -> Result<(), DesktopShellError> {
        let root = root.as_ref();
        if self
            .pending_vault_repair
            .as_ref()
            .is_some_and(|proposal| proposal.root() == root)
        {
            self.pending_vault_repair = None;
            return Ok(());
        }

        Err(DesktopShellError::NoPendingVaultRepair(root.to_path_buf()))
    }

    pub fn confirm_vault_repair(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<ActiveVault, DesktopShellError> {
        let root = root.as_ref();
        let proposal = self
            .pending_vault_repair
            .as_ref()
            .filter(|proposal| proposal.root() == root)
            .cloned()
            .ok_or_else(|| DesktopShellError::NoPendingVaultRepair(root.to_path_buf()))?;
        let vault = proposal.confirm().map_err(DesktopShellError::Vault)?;
        self.set_active_vault(vault)
            .map_err(DesktopShellError::Vault)
    }

    pub fn switch_active_vault(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<ActiveVault, VaultError> {
        self.open_vault(root)
    }

    pub fn active_vault(&self) -> Option<ActiveVault> {
        self.active_vault
            .as_ref()
            .map(|vault| ActiveVault::from(vault.root()))
    }

    pub fn known_vaults(&self) -> Result<Vec<KnownVault>, DesktopShellError> {
        let roots = if let Some(app_state_dir) = self.app_state_dir.as_ref() {
            read_known_vault_roots(app_state_dir).map_err(DesktopShellError::Io)?
        } else {
            self.known_vault_roots.clone()
        };

        Ok(roots.into_iter().map(|root| KnownVault { root }).collect())
    }

    pub fn startup_state(&self) -> Result<DesktopStartup, DesktopShellError> {
        Ok(DesktopStartup {
            active_vault: self.active_vault(),
            known_vaults: self.known_vaults()?,
            repair_proposal: self.pending_vault_repair.clone(),
            notice: self.startup_notice.clone(),
        })
    }

    pub fn add_artwork_item(&self, item: AddArtworkItem) -> Result<SavedItem, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .add_artwork_item(item)
            .map_err(DesktopShellError::Vault)
    }

    pub fn add_artwork_files(
        &self,
        source_files: impl IntoIterator<Item = PathBuf>,
        metadata: ArtworkImportMetadata,
    ) -> Result<Vec<SavedItem>, DesktopShellError> {
        self.add_artwork_files_with_options(source_files, metadata, ExactDuplicatePolicy::Skip)
            .map(|summary| summary.imported_items().to_vec())
    }

    pub fn add_artwork_files_with_options(
        &self,
        source_files: impl IntoIterator<Item = PathBuf>,
        metadata: ArtworkImportMetadata,
        exact_duplicate_policy: ExactDuplicatePolicy,
    ) -> Result<SelectedFileImportSummary, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .add_artwork_files_with_options(
                source_files,
                ArtworkImportOptions {
                    metadata,
                    exact_duplicate_policy,
                },
            )
            .map_err(DesktopShellError::Vault)
    }

    pub fn import_paintings_folder(
        &self,
        source_folder: impl AsRef<Path>,
    ) -> Result<Vec<SavedItem>, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .import_paintings_folder(source_folder)
            .map_err(DesktopShellError::Vault)
    }

    pub fn run_paintings_import<F>(
        &self,
        source_folder: impl AsRef<Path>,
        on_progress: F,
    ) -> Result<ImportRunSummary, DesktopShellError>
    where
        F: FnMut(&ImportProgress) -> bool,
    {
        self.run_paintings_import_with_metadata(
            source_folder,
            ArtworkImportMetadata::default(),
            ExactDuplicatePolicy::Skip,
            on_progress,
        )
    }

    pub fn run_paintings_import_with_metadata<F>(
        &self,
        source_folder: impl AsRef<Path>,
        metadata: ArtworkImportMetadata,
        exact_duplicate_policy: ExactDuplicatePolicy,
        mut on_progress: F,
    ) -> Result<ImportRunSummary, DesktopShellError>
    where
        F: FnMut(&ImportProgress) -> bool,
    {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .run_paintings_import_with_options(
                source_folder,
                ArtworkImportOptions {
                    metadata,
                    exact_duplicate_policy,
                },
                |progress| {
                    if on_progress(progress) {
                        ImportRunAction::Cancel
                    } else {
                        ImportRunAction::Continue
                    }
                },
            )
            .map_err(DesktopShellError::Vault)
    }

    pub fn search_metadata(&self, query: &str) -> Result<Vec<SearchResult>, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .search_metadata(query)
            .map_err(DesktopShellError::Vault)
    }

    pub fn open_saved_item(&self, id: &str) -> Result<SavedItem, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault.open_saved_item(id).map_err(DesktopShellError::Vault)
    }

    pub fn browse_artwork_items(
        &self,
        home_subvault: &str,
    ) -> Result<Vec<ArtworkGridItem>, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .browse_artwork_items(home_subvault)
            .map_err(DesktopShellError::Vault)
    }

    pub fn browse_idea_sources(&self) -> Result<Vec<IdeaSourceListItem>, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .browse_idea_sources()
            .map_err(DesktopShellError::Vault)
    }

    pub fn review_queue(&self) -> Result<Vec<ReviewQueueItem>, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault.review_queue().map_err(DesktopShellError::Vault)
    }

    pub fn workbench_snapshot(
        &self,
        request: WorkbenchRequest,
    ) -> Result<WorkbenchSnapshot, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        let search_results = match request.search_query.as_deref().map(str::trim) {
            Some(query) if !query.is_empty() => vault
                .search_metadata(query)
                .map_err(DesktopShellError::Vault)?,
            _ => Vec::new(),
        };
        let artwork_items = vault
            .browse_artwork_items_sorted(&request.home_subvault, request.artwork_sort)
            .map_err(DesktopShellError::Vault)?;
        let selected_item = match request.selected_item_id.as_deref() {
            Some(id) => match vault.item_details(id) {
                Ok(details) => Some(details),
                Err(VaultError::MalformedItemRecord(_) | VaultError::SavedItemNotFound(_)) => None,
                Err(error) => return Err(DesktopShellError::Vault(error)),
            },
            None => None,
        };

        Ok(WorkbenchSnapshot {
            active_vault: ActiveVault::from(vault.root()),
            subvaults: vault.list_subvaults().map_err(DesktopShellError::Vault)?,
            collections: vault.list_collections().map_err(DesktopShellError::Vault)?,
            artwork_items,
            idea_sources: vault
                .browse_idea_sources()
                .map_err(DesktopShellError::Vault)?,
            review_queue: vault.review_queue().map_err(DesktopShellError::Vault)?,
            search_results,
            selected_item,
            vault_problems: vault.vault_problems().map_err(DesktopShellError::Vault)?,
        })
    }

    pub fn refresh_workbench(
        &self,
        request: WorkbenchRequest,
    ) -> Result<WorkbenchSnapshot, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .rebuild_metadata_index()
            .map_err(DesktopShellError::Vault)?;
        self.workbench_snapshot(request)
    }

    pub fn activity_log_path(&self) -> Result<PathBuf, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        Ok(vault.activity_log_path())
    }

    pub fn item_details(&self, id: &str) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        match vault.item_details(id) {
            Ok(details) => Ok(details),
            Err(error) => {
                let message = error.to_string();
                vault
                    .record_error_event("item-details", id, &message)
                    .map_err(DesktopShellError::Vault)?;
                Err(DesktopShellError::Vault(error))
            }
        }
    }

    pub fn update_item_record(
        &self,
        update: UpdateItemRecord,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .update_item_record(update)
            .map_err(DesktopShellError::Vault)
    }

    pub fn save_item_record_edit(
        &self,
        edit: ItemRecordEdit,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .save_item_record_edit(edit)
            .map_err(DesktopShellError::Vault)
    }

    pub fn resolve_review_reason(
        &self,
        resolution: ReviewReasonResolution,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .resolve_review_reason(resolution)
            .map_err(DesktopShellError::Vault)
    }

    pub fn confirm_item_folder_rename(
        &self,
        id: &str,
        proposal: &ItemFolderRenameProposal,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;
        vault
            .confirm_item_folder_rename(id, proposal)
            .map_err(DesktopShellError::Vault)
    }

    pub fn manual_fallback_capture(
        &self,
        capture: ManualFallbackCapture,
    ) -> Result<SavedItem, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .manual_fallback_capture(capture)
            .map_err(DesktopShellError::Vault)
    }

    pub fn capture_extracted_text(
        &self,
        capture: ExtractedTextCapture,
    ) -> Result<SavedItem, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .capture_extracted_text(capture)
            .map_err(DesktopShellError::Vault)
    }

    pub fn capture_source_link(
        &self,
        capture: SourceLinkCapture,
        extractor: &dyn SourceExtractor,
    ) -> Result<SourceCaptureResult, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .capture_source_link(capture, extractor)
            .map_err(DesktopShellError::Vault)
    }

    pub fn upsert_tag(&self, tag: TagDefinition) -> Result<(), DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault.upsert_tag(tag).map_err(DesktopShellError::Vault)
    }

    pub fn add_tags_to_item(
        &self,
        id: &str,
        tags: Vec<String>,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .add_tags_to_item(id, tags)
            .map_err(DesktopShellError::Vault)
    }

    pub fn create_collection(
        &self,
        collection: CollectionDefinition,
    ) -> Result<Collection, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .create_collection(collection)
            .map_err(DesktopShellError::Vault)
    }

    pub fn add_item_to_collection(
        &self,
        collection_id: &str,
        item_id: &str,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .add_item_to_collection(collection_id, item_id)
            .map_err(DesktopShellError::Vault)
    }

    pub fn add_item_link(
        &self,
        item_id: &str,
        link: ItemLinkDefinition,
    ) -> Result<ItemDetails, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .add_item_link(item_id, link)
            .map_err(DesktopShellError::Vault)
    }

    pub fn enrich_idea_with_ai(
        &self,
        id: &str,
        budget_mode: AiBudgetMode,
        provider: &dyn AiProvider,
    ) -> Result<AiEnrichmentResult, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .enrich_idea_with_ai(id, budget_mode, provider)
            .map_err(DesktopShellError::Vault)
    }

    pub fn suggest_artwork_metadata_with_ai(
        &self,
        id: &str,
        budget_mode: AiBudgetMode,
        provider: &dyn AiProvider,
    ) -> Result<AiEnrichmentResult, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .suggest_artwork_metadata_with_ai(id, budget_mode, provider)
            .map_err(DesktopShellError::Vault)
    }

    pub fn configure_openai_provider(
        &self,
        config: OpenAiProviderConfig,
    ) -> Result<(), DesktopShellError> {
        let app_state_dir = self
            .app_state_dir
            .as_ref()
            .ok_or(DesktopShellError::AppStateNotConfigured)?;
        fs::create_dir_all(app_state_dir).map_err(DesktopShellError::Io)?;
        fs::write(
            app_state_dir.join(OPENAI_PROVIDER_FILE),
            openai_provider_config_toml(&config),
        )
        .map_err(DesktopShellError::Io)
    }

    pub fn openai_provider_config(&self) -> Result<OpenAiProviderConfig, DesktopShellError> {
        let app_state_dir = self
            .app_state_dir
            .as_ref()
            .ok_or(DesktopShellError::AppStateNotConfigured)?;
        let path = app_state_dir.join(OPENAI_PROVIDER_FILE);
        parse_openai_provider_config(
            &path,
            &fs::read_to_string(&path).map_err(DesktopShellError::Io)?,
        )
    }

    fn set_active_vault(&mut self, vault: Vault) -> Result<ActiveVault, VaultError> {
        let active_vault = ActiveVault::from(vault.root());
        self.remember_vault_root(vault.root())?;
        if let Some(app_state_dir) = self.app_state_dir.as_ref() {
            write_last_active_vault_root(app_state_dir, vault.root())?;
        }
        self.active_vault = Some(vault);
        self.pending_vault_repair = None;
        self.startup_notice = None;
        Ok(active_vault)
    }

    fn remember_vault_root(&mut self, root: &Path) -> Result<(), VaultError> {
        if !self.known_vault_roots.iter().any(|known| known == root) {
            self.known_vault_roots.push(root.to_path_buf());
        }

        if let Some(app_state_dir) = self.app_state_dir.as_ref() {
            write_known_vault_roots(app_state_dir, &self.known_vault_roots)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchRequest {
    pub home_subvault: String,
    pub artwork_sort: ArtworkSort,
    pub search_query: Option<String>,
    pub selected_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchSnapshot {
    active_vault: ActiveVault,
    subvaults: Vec<String>,
    collections: Vec<Collection>,
    artwork_items: Vec<ArtworkGridItem>,
    idea_sources: Vec<IdeaSourceListItem>,
    review_queue: Vec<ReviewQueueItem>,
    search_results: Vec<SearchResult>,
    selected_item: Option<ItemDetails>,
    vault_problems: Vec<VaultProblem>,
}

impl WorkbenchSnapshot {
    pub fn active_vault(&self) -> &ActiveVault {
        &self.active_vault
    }

    pub fn subvaults(&self) -> &[String] {
        &self.subvaults
    }

    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }

    pub fn artwork_items(&self) -> &[ArtworkGridItem] {
        &self.artwork_items
    }

    pub fn idea_sources(&self) -> &[IdeaSourceListItem] {
        &self.idea_sources
    }

    pub fn review_queue(&self) -> &[ReviewQueueItem] {
        &self.review_queue
    }

    pub fn search_results(&self) -> &[SearchResult] {
        &self.search_results
    }

    pub fn selected_item(&self) -> Option<&ItemDetails> {
        self.selected_item.as_ref()
    }

    pub fn vault_problems(&self) -> &[VaultProblem] {
        &self.vault_problems
    }
}

#[derive(Debug, Default)]
pub struct TauriCommandState {
    shell: DesktopShell,
}

impl TauriCommandState {
    pub fn with_app_state_dir(app_state_dir: impl AsRef<Path>) -> Self {
        Self {
            shell: DesktopShell::with_app_state_dir(app_state_dir),
        }
    }

    pub fn startup(&self) -> Result<DesktopStartupView, DesktopShellError> {
        self.shell.startup_state().map(DesktopStartupView::from)
    }

    pub fn create_vault(&mut self, root: String) -> Result<ActiveVaultView, DesktopShellError> {
        self.shell
            .create_vault(root)
            .map(ActiveVaultView::from)
            .map_err(DesktopShellError::Vault)
    }

    pub fn open_vault(&mut self, root: String) -> Result<OpenVaultView, DesktopShellError> {
        self.shell
            .request_open_vault(root)
            .map(OpenVaultView::from)
            .map_err(DesktopShellError::Vault)
    }

    pub fn confirm_vault_repair(
        &mut self,
        root: String,
    ) -> Result<ActiveVaultView, DesktopShellError> {
        self.shell
            .confirm_vault_repair(root)
            .map(ActiveVaultView::from)
    }

    pub fn cancel_vault_repair(&mut self, root: String) -> Result<(), DesktopShellError> {
        self.shell.cancel_vault_repair(root)
    }

    pub fn import_paintings(
        &self,
        command: ImportPaintingsCommand,
    ) -> Result<Vec<SavedItemView>, DesktopShellError> {
        self.shell
            .import_paintings_folder(command.source_folder)
            .map(|items| items.into_iter().map(SavedItemView::from).collect())
    }

    pub fn run_paintings_import<F>(
        &self,
        command: RunPaintingsImportCommand,
        mut on_progress: F,
    ) -> Result<ImportRunSummaryView, DesktopShellError>
    where
        F: FnMut(&ImportProgressView) -> bool,
    {
        self.shell
            .run_paintings_import_with_metadata(
                command.source_folder,
                ArtworkImportMetadata {
                    creator: non_empty(command.creator),
                    year: non_empty(command.year),
                    saving_reason: non_empty(command.saving_reason),
                },
                if command.import_exact_duplicates {
                    ExactDuplicatePolicy::ImportAnyway
                } else {
                    ExactDuplicatePolicy::Skip
                },
                |progress| on_progress(&ImportProgressView::from(progress)),
            )
            .map(ImportRunSummaryView::from)
    }

    pub fn add_artwork_files(
        &self,
        command: AddArtworkFilesCommand,
    ) -> Result<SelectedFileImportSummaryView, DesktopShellError> {
        self.shell
            .add_artwork_files_with_options(
                command.source_files.into_iter().map(PathBuf::from),
                ArtworkImportMetadata {
                    creator: non_empty(command.creator),
                    year: non_empty(command.year),
                    saving_reason: non_empty(command.saving_reason),
                },
                if command.import_exact_duplicates {
                    ExactDuplicatePolicy::ImportAnyway
                } else {
                    ExactDuplicatePolicy::Skip
                },
            )
            .map(ArtworkImportOutcomeView::from)
    }

    pub fn capture_idea(
        &self,
        command: CaptureIdeaCommand,
    ) -> Result<SavedItemView, DesktopShellError> {
        self.shell
            .manual_fallback_capture(ManualFallbackCapture {
                source_link: command.source_link,
                title: command.title,
                saving_reason: command.saving_reason,
                copied_text: command.copied_text,
                copied_image: None,
            })
            .map(SavedItemView::from)
    }

    pub fn workbench_snapshot(
        &self,
        command: WorkbenchSnapshotCommand,
    ) -> Result<WorkbenchSnapshotView, DesktopShellError> {
        self.shell
            .workbench_snapshot(workbench_request(command)?)
            .map(WorkbenchSnapshotView::from)
    }

    pub fn refresh_workbench(
        &self,
        command: WorkbenchSnapshotCommand,
    ) -> Result<WorkbenchSnapshotView, DesktopShellError> {
        self.shell
            .refresh_workbench(workbench_request(command)?)
            .map(WorkbenchSnapshotView::from)
    }

    pub fn activity_log_path(&self) -> Result<String, DesktopShellError> {
        self.shell
            .activity_log_path()
            .map(|path| path_string(&path))
    }

    pub fn save_item_record(
        &self,
        command: SaveItemRecordCommand,
    ) -> Result<ItemRecordSaveView, DesktopShellError> {
        let id = command.id.clone();
        match self.shell.save_item_record_edit(command.into()) {
            Ok(item) => Ok(ItemRecordSaveView::Saved {
                item: ItemDetailsView::from(&item),
            }),
            Err(DesktopShellError::Vault(VaultError::ItemRecordConflict { .. })) => {
                let external = self.shell.item_details(&id)?;
                Ok(ItemRecordSaveView::Conflict {
                    external_item: ItemDetailsView::from(&external),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn resolve_review_reason(
        &self,
        command: ResolveReviewReasonCommand,
    ) -> Result<ItemDetailsView, DesktopShellError> {
        let action = match command.action.as_str() {
            "accept" => ReviewReasonAction::Accept,
            "correct" => ReviewReasonAction::Correct {
                value: command.correction.unwrap_or_default(),
            },
            "dismiss" => ReviewReasonAction::Dismiss,
            action => {
                return Err(DesktopShellError::InvalidReviewReasonAction(
                    action.to_string(),
                ))
            }
        };
        self.shell
            .resolve_review_reason(ReviewReasonResolution {
                item_id: command.item_id,
                reason_id: command.reason_id,
                expected_revision: command.expected_revision,
                action,
            })
            .map(|item| ItemDetailsView::from(&item))
    }

    pub fn confirm_item_folder_rename(
        &self,
        command: ConfirmItemFolderRenameCommand,
    ) -> Result<ItemDetailsView, DesktopShellError> {
        self.shell
            .confirm_item_folder_rename(
                &command.id,
                &ItemFolderRenameProposal::reviewed(command.current_path, command.proposed_path),
            )
            .map(|item| ItemDetailsView::from(&item))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPaintingsCommand {
    pub source_folder: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPaintingsImportCommand {
    pub source_folder: String,
    pub creator: Option<String>,
    pub year: Option<String>,
    pub saving_reason: Option<String>,
    pub import_exact_duplicates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveItemRecordCommand {
    pub id: String,
    pub expected_revision: String,
    pub overwrite_conflict: bool,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub saving_reason: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveReviewReasonCommand {
    pub item_id: String,
    pub reason_id: String,
    pub expected_revision: String,
    pub action: String,
    pub correction: Option<String>,
}

impl From<SaveItemRecordCommand> for ItemRecordEdit {
    fn from(command: SaveItemRecordCommand) -> Self {
        Self {
            id: command.id,
            expected_revision: command.expected_revision,
            overwrite_conflict: command.overwrite_conflict,
            title: command.title,
            creator: command.creator,
            year: command.year,
            saving_reason: command.saving_reason,
            summary: command.summary,
            tags: command.tags,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmItemFolderRenameCommand {
    pub id: String,
    pub current_path: String,
    pub proposed_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportProgressView {
    pub processed: usize,
    pub total: usize,
    pub current_file: String,
}

impl From<&ImportProgress> for ImportProgressView {
    fn from(progress: &ImportProgress) -> Self {
        Self {
            processed: progress.processed(),
            total: progress.total(),
            current_file: progress.current_file().display().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportSkippedEntryView {
    pub path: String,
    pub reason: String,
    pub existing_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportDuplicateCandidateEntryView {
    pub path: String,
    pub item_id: String,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportFailedEntryView {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportVaultProblemView {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtworkImportOutcomeView {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub duplicate_candidate_count: usize,
    pub exact_duplicate_count: usize,
    pub cancelled_count: usize,
    pub failed_count: usize,
    pub cancelled: bool,
    pub imported_items: Vec<SavedItemView>,
    pub skipped_entries: Vec<ImportSkippedEntryView>,
    pub duplicate_candidate_entries: Vec<ImportDuplicateCandidateEntryView>,
    pub cancelled_files: Vec<String>,
    pub failed_entries: Vec<ImportFailedEntryView>,
    pub maintenance_errors: Vec<String>,
    pub vault_problems: Vec<ImportVaultProblemView>,
}

pub type ImportRunSummaryView = ArtworkImportOutcomeView;
pub type SelectedFileImportSummaryView = ArtworkImportOutcomeView;

impl From<ImportRunSummary> for ArtworkImportOutcomeView {
    fn from(summary: ImportRunSummary) -> Self {
        Self::from_outcome(&summary)
    }
}

impl From<SelectedFileImportSummary> for ArtworkImportOutcomeView {
    fn from(summary: SelectedFileImportSummary) -> Self {
        Self::from_outcome(&summary)
    }
}

impl ArtworkImportOutcomeView {
    fn from_outcome(summary: &ArtworkImportOutcome) -> Self {
        Self {
            imported_count: summary.imported_count(),
            skipped_count: summary.skipped_count(),
            duplicate_candidate_count: summary.duplicate_candidate_count(),
            exact_duplicate_count: summary.exact_duplicate_count(),
            cancelled_count: summary.cancelled_count(),
            failed_count: summary.failed_count(),
            cancelled: summary.was_cancelled(),
            imported_items: summary
                .imported_items()
                .iter()
                .cloned()
                .map(SavedItemView::from)
                .collect(),
            skipped_entries: summary
                .skipped_entries()
                .iter()
                .map(|entry| ImportSkippedEntryView {
                    path: entry.path().display().to_string(),
                    reason: entry.reason().to_string(),
                    existing_item_id: entry.existing_item_id().map(ToOwned::to_owned),
                })
                .collect(),
            duplicate_candidate_entries: summary
                .duplicate_candidate_entries()
                .iter()
                .map(|entry| ImportDuplicateCandidateEntryView {
                    path: entry.path().display().to_string(),
                    item_id: entry.item_id().to_string(),
                    candidate_count: entry.candidate_count(),
                })
                .collect(),
            cancelled_files: summary
                .cancelled_files()
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            failed_entries: summary
                .failed_entries()
                .iter()
                .map(|entry| ImportFailedEntryView {
                    path: entry.path().display().to_string(),
                    error: entry.error().to_string(),
                })
                .collect(),
            maintenance_errors: summary.maintenance_errors().to_vec(),
            vault_problems: summary
                .vault_problems()
                .iter()
                .map(|problem| ImportVaultProblemView {
                    path: problem.path().display().to_string(),
                    error: problem.error().to_string(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddArtworkFilesCommand {
    pub source_files: Vec<String>,
    pub creator: Option<String>,
    pub year: Option<String>,
    pub saving_reason: Option<String>,
    pub import_exact_duplicates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureIdeaCommand {
    pub source_link: String,
    pub title: String,
    pub saving_reason: Option<String>,
    pub copied_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchSnapshotCommand {
    pub home_subvault: String,
    pub artwork_sort: String,
    pub search_query: Option<String>,
    pub selected_item_id: Option<String>,
}

fn workbench_request(
    command: WorkbenchSnapshotCommand,
) -> Result<WorkbenchRequest, DesktopShellError> {
    Ok(WorkbenchRequest {
        home_subvault: command.home_subvault,
        artwork_sort: parse_artwork_sort(&command.artwork_sort)?,
        search_query: command.search_query,
        selected_item_id: command.selected_item_id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActiveVaultView {
    pub root: String,
}

impl From<ActiveVault> for ActiveVaultView {
    fn from(vault: ActiveVault) -> Self {
        Self {
            root: path_string(vault.root()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VaultRepairProposalView {
    pub root: String,
    pub directories: Vec<String>,
}

impl From<&VaultRepairProposal> for VaultRepairProposalView {
    fn from(proposal: &VaultRepairProposal) -> Self {
        Self {
            root: path_string(proposal.root()),
            directories: proposal
                .directories()
                .iter()
                .map(|path| path_string(path))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OpenVaultView {
    Opened { vault: ActiveVaultView },
    RepairRequired { proposal: VaultRepairProposalView },
}

impl From<OpenVaultResult> for OpenVaultView {
    fn from(result: OpenVaultResult) -> Self {
        match result {
            OpenVaultResult::Opened(vault) => Self::Opened {
                vault: ActiveVaultView::from(vault),
            },
            OpenVaultResult::RepairRequired(proposal) => Self::RepairRequired {
                proposal: VaultRepairProposalView::from(&proposal),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnownVaultView {
    pub root: String,
}

impl From<&KnownVault> for KnownVaultView {
    fn from(vault: &KnownVault) -> Self {
        Self {
            root: path_string(vault.root()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopStartupView {
    pub active_vault: Option<ActiveVaultView>,
    pub known_vaults: Vec<KnownVaultView>,
    pub repair_proposal: Option<VaultRepairProposalView>,
    pub notice: Option<String>,
}

impl From<DesktopStartup> for DesktopStartupView {
    fn from(startup: DesktopStartup) -> Self {
        Self {
            active_vault: startup.active_vault().cloned().map(ActiveVaultView::from),
            known_vaults: startup
                .known_vaults()
                .iter()
                .map(KnownVaultView::from)
                .collect(),
            repair_proposal: startup.repair_proposal().map(VaultRepairProposalView::from),
            notice: startup.notice().map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedItemView {
    pub id: String,
    pub home_subvault: String,
    pub item_folder: String,
}

impl From<SavedItem> for SavedItemView {
    fn from(item: SavedItem) -> Self {
        Self {
            id: item.id().to_string(),
            home_subvault: item.home_subvault().to_string(),
            item_folder: path_string(item.item_folder()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CollectionView {
    pub id: String,
    pub name: String,
}

impl From<&Collection> for CollectionView {
    fn from(collection: &Collection) -> Self {
        Self {
            id: collection.id().to_string(),
            name: collection.name().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtworkGridItemView {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub primary_file: String,
    pub thumbnail_file: String,
    pub thumbnail_is_placeholder: bool,
    pub review_status: String,
}

impl From<&ArtworkGridItem> for ArtworkGridItemView {
    fn from(item: &ArtworkGridItem) -> Self {
        Self {
            id: item.saved_item().id().to_string(),
            title: item.title().to_string(),
            creator: item.creator().to_string(),
            year: item.year().to_string(),
            primary_file: path_string(item.primary_file()),
            thumbnail_file: path_string(item.thumbnail_file()),
            thumbnail_is_placeholder: item.thumbnail_is_placeholder(),
            review_status: item.review_status().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdeaSourceItemView {
    pub id: String,
    pub title: String,
    pub source_link: String,
    pub source_copy: Option<String>,
    pub review_status: String,
    pub saving_reason: Option<String>,
}

impl From<&IdeaSourceListItem> for IdeaSourceItemView {
    fn from(item: &IdeaSourceListItem) -> Self {
        Self {
            id: item.saved_item().id().to_string(),
            title: item.title().to_string(),
            source_link: item.source_link().to_string(),
            source_copy: item.source_copy().map(path_string),
            review_status: item.review_status().to_string(),
            saving_reason: item.saving_reason().map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewQueueItemView {
    pub id: String,
    pub home_subvault: String,
    pub item_type: String,
    pub title: String,
    pub review_status: String,
    pub saving_reason: Option<String>,
    pub review_reasons: Vec<ReviewReasonView>,
}

impl From<&ReviewQueueItem> for ReviewQueueItemView {
    fn from(item: &ReviewQueueItem) -> Self {
        Self {
            id: item.saved_item().id().to_string(),
            home_subvault: item.home_subvault().to_string(),
            item_type: item.item_type().to_string(),
            title: item.title().to_string(),
            review_status: item.review_status().to_string(),
            saving_reason: item.saving_reason().map(str::to_string),
            review_reasons: item
                .review_reasons()
                .iter()
                .map(ReviewReasonView::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewReasonView {
    pub id: String,
    pub kind: String,
    pub target_field: Option<String>,
    pub message: String,
    pub evidence: String,
}

impl From<&ReviewReason> for ReviewReasonView {
    fn from(reason: &ReviewReason) -> Self {
        Self {
            id: reason.id().to_string(),
            kind: reason.kind().to_string(),
            target_field: reason.target_field().map(str::to_string),
            message: reason.message().to_string(),
            evidence: reason.evidence().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResultView {
    pub id: String,
    pub home_subvault: String,
    pub title: String,
}

impl From<&SearchResult> for SearchResultView {
    fn from(result: &SearchResult) -> Self {
        Self {
            id: result.saved_item().id().to_string(),
            home_subvault: result.saved_item().home_subvault().to_string(),
            title: result.title().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemDetailsView {
    pub id: String,
    pub home_subvault: String,
    pub item_folder: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub primary_file: String,
    pub review_status: String,
    pub review_reasons: Vec<ReviewReasonView>,
    pub tags: Vec<String>,
    pub collections: Vec<String>,
    pub item_links: Vec<ItemLinkView>,
    pub saving_reason: Option<String>,
    pub source_link: Option<String>,
    pub summary: Option<String>,
    pub source_copy: Option<String>,
    pub record_revision: String,
    pub folder_rename_proposal: Option<ItemFolderRenameProposalView>,
}

impl From<&ItemDetails> for ItemDetailsView {
    fn from(details: &ItemDetails) -> Self {
        Self {
            id: details.id().to_string(),
            home_subvault: details.home_subvault().to_string(),
            item_folder: path_string(details.item_folder()),
            title: details.title().to_string(),
            creator: details.creator().to_string(),
            year: details.year().to_string(),
            primary_file: path_string(details.primary_file()),
            review_status: details.review_status().to_string(),
            review_reasons: details
                .review_reasons()
                .iter()
                .map(ReviewReasonView::from)
                .collect(),
            tags: details.tags().into_iter().map(str::to_string).collect(),
            collections: details
                .collections()
                .into_iter()
                .map(str::to_string)
                .collect(),
            item_links: details
                .item_links()
                .iter()
                .map(|link| ItemLinkView {
                    link_type: link.link_type().to_string(),
                    target: link.target().to_string(),
                    label: link.label().to_string(),
                })
                .collect(),
            saving_reason: details.saving_reason().map(str::to_string),
            source_link: details.source_link().map(str::to_string),
            summary: details.summary().map(str::to_string),
            source_copy: details.source_copy().map(path_string),
            record_revision: details.record_revision().to_string(),
            folder_rename_proposal: details
                .folder_rename_proposal()
                .map(ItemFolderRenameProposalView::from),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemLinkView {
    pub link_type: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemFolderRenameProposalView {
    pub current_path: String,
    pub proposed_path: String,
}

impl From<&ItemFolderRenameProposal> for ItemFolderRenameProposalView {
    fn from(proposal: &ItemFolderRenameProposal) -> Self {
        Self {
            current_path: path_string(proposal.current_path()),
            proposed_path: path_string(proposal.proposed_path()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ItemRecordSaveView {
    Saved { item: ItemDetailsView },
    Conflict { external_item: ItemDetailsView },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchSnapshotView {
    pub active_vault: ActiveVaultView,
    pub subvaults: Vec<String>,
    pub collections: Vec<CollectionView>,
    pub artwork_items: Vec<ArtworkGridItemView>,
    pub idea_sources: Vec<IdeaSourceItemView>,
    pub review_queue: Vec<ReviewQueueItemView>,
    pub search_results: Vec<SearchResultView>,
    pub selected_item: Option<ItemDetailsView>,
    pub vault_problems: Vec<VaultProblemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VaultProblemView {
    pub path: String,
    pub error: String,
}

impl From<&VaultProblem> for VaultProblemView {
    fn from(problem: &VaultProblem) -> Self {
        Self {
            path: path_string(problem.path()),
            error: problem.error().to_string(),
        }
    }
}

impl From<WorkbenchSnapshot> for WorkbenchSnapshotView {
    fn from(snapshot: WorkbenchSnapshot) -> Self {
        Self {
            active_vault: ActiveVaultView::from(snapshot.active_vault().clone()),
            subvaults: snapshot.subvaults().to_vec(),
            collections: snapshot
                .collections()
                .iter()
                .map(CollectionView::from)
                .collect(),
            artwork_items: snapshot
                .artwork_items()
                .iter()
                .map(ArtworkGridItemView::from)
                .collect(),
            idea_sources: snapshot
                .idea_sources()
                .iter()
                .map(IdeaSourceItemView::from)
                .collect(),
            review_queue: snapshot
                .review_queue()
                .iter()
                .map(ReviewQueueItemView::from)
                .collect(),
            search_results: snapshot
                .search_results()
                .iter()
                .map(SearchResultView::from)
                .collect(),
            selected_item: snapshot.selected_item().map(ItemDetailsView::from),
            vault_problems: snapshot
                .vault_problems()
                .iter()
                .map(VaultProblemView::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenVaultResult {
    Opened(ActiveVault),
    RepairRequired(VaultRepairProposal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveVault {
    root: PathBuf,
}

impl ActiveVault {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl From<&Path> for ActiveVault {
    fn from(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownVault {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopStartup {
    active_vault: Option<ActiveVault>,
    known_vaults: Vec<KnownVault>,
    repair_proposal: Option<VaultRepairProposal>,
    notice: Option<String>,
}

impl DesktopStartup {
    pub fn active_vault(&self) -> Option<&ActiveVault> {
        self.active_vault.as_ref()
    }

    pub fn known_vaults(&self) -> &[KnownVault] {
        &self.known_vaults
    }

    pub fn repair_proposal(&self) -> Option<&VaultRepairProposal> {
        self.repair_proposal.as_ref()
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
}

impl KnownVault {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiProviderConfig {
    pub api_key: String,
    pub model: String,
}

impl OpenAiProviderConfig {
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Debug)]
pub enum DesktopShellError {
    NoActiveVault,
    NoPendingVaultRepair(PathBuf),
    AppStateNotConfigured,
    MalformedProviderConfig(PathBuf),
    UnsupportedArtworkSort(String),
    InvalidReviewReasonAction(String),
    Io(io::Error),
    Vault(VaultError),
}

impl std::fmt::Display for DesktopShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoActiveVault => write!(f, "no active vault is open"),
            Self::NoPendingVaultRepair(path) => {
                write!(f, "no vault repair is pending for: {}", path.display())
            }
            Self::AppStateNotConfigured => write!(f, "app state directory is not configured"),
            Self::MalformedProviderConfig(path) => {
                write!(f, "provider config is malformed: {}", path.display())
            }
            Self::UnsupportedArtworkSort(sort) => write!(f, "unsupported artwork sort: {sort}"),
            Self::InvalidReviewReasonAction(action) => {
                write!(f, "invalid review reason action: {action}")
            }
            Self::Io(error) => write!(f, "{error}"),
            Self::Vault(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DesktopShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoActiveVault => None,
            Self::NoPendingVaultRepair(_) => None,
            Self::AppStateNotConfigured => None,
            Self::MalformedProviderConfig(_) => None,
            Self::UnsupportedArtworkSort(_) => None,
            Self::InvalidReviewReasonAction(_) => None,
            Self::Io(error) => Some(error),
            Self::Vault(error) => Some(error),
        }
    }
}

fn openai_provider_config_toml(config: &OpenAiProviderConfig) -> String {
    format!(
        "api_key = \"{}\"\nmodel = \"{}\"\n",
        escape_toml_string(&config.api_key),
        escape_toml_string(&config.model)
    )
}

fn parse_openai_provider_config(
    path: &Path,
    contents: &str,
) -> Result<OpenAiProviderConfig, DesktopShellError> {
    let api_key = toml_string_value(contents, "api_key")
        .ok_or_else(|| DesktopShellError::MalformedProviderConfig(path.to_path_buf()))?;
    let model = toml_string_value(contents, "model")
        .ok_or_else(|| DesktopShellError::MalformedProviderConfig(path.to_path_buf()))?;

    Ok(OpenAiProviderConfig { api_key, model })
}

fn toml_string_value(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    contents.lines().find_map(|line| {
        let value = line.strip_prefix(&prefix)?.strip_suffix('"')?;
        Some(value.replace("\\\"", "\"").replace("\\\\", "\\"))
    })
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn parse_artwork_sort(sort: &str) -> Result<ArtworkSort, DesktopShellError> {
    match sort {
        "newest" => Ok(ArtworkSort::Newest),
        "oldest" => Ok(ArtworkSort::Oldest),
        "title" => Ok(ArtworkSort::Title),
        "creator" => Ok(ArtworkSort::Creator),
        "year" => Ok(ArtworkSort::Year),
        _ => Err(DesktopShellError::UnsupportedArtworkSort(sort.to_string())),
    }
}

fn read_known_vault_roots(app_state_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let path = app_state_dir.join(KNOWN_VAULTS_FILE);
    if !path.is_file() {
        return Ok(Vec::new());
    }

    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

fn write_known_vault_roots(app_state_dir: &Path, roots: &[PathBuf]) -> Result<(), VaultError> {
    fs::create_dir_all(app_state_dir)?;
    let mut contents = String::new();
    for root in roots {
        contents.push_str(&root.display().to_string());
        contents.push('\n');
    }
    fs::write(app_state_dir.join(KNOWN_VAULTS_FILE), contents)?;
    Ok(())
}

fn read_last_active_vault_root(app_state_dir: &Path) -> io::Result<Option<PathBuf>> {
    let path = app_state_dir.join(LAST_ACTIVE_VAULT_FILE);
    if !path.is_file() {
        return Ok(None);
    }

    let root = fs::read_to_string(path)?;
    let root = root.trim_end();
    if root.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(root)))
}

fn write_last_active_vault_root(app_state_dir: &Path, root: &Path) -> Result<(), VaultError> {
    fs::create_dir_all(app_state_dir)?;
    fs::write(
        app_state_dir.join(LAST_ACTIVE_VAULT_FILE),
        root.display().to_string(),
    )?;
    Ok(())
}
