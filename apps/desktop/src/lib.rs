use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const KNOWN_VAULTS_FILE: &str = "known-vaults.tsv";
const OPENAI_PROVIDER_FILE: &str = "openai-provider.toml";

use gruenes_gewolbe_core::{
    AddArtworkItem, AiBudgetMode, AiEnrichmentResult, AiProvider, ArtworkGridItem, Collection,
    CollectionDefinition, ExtractedTextCapture, IdeaSourceListItem, ItemDetails,
    ItemLinkDefinition, ManualFallbackCapture, ReviewQueueItem, SavedItem, SearchResult,
    SourceCaptureResult, SourceExtractor, SourceLinkCapture, TagDefinition, UpdateItemRecord,
    Vault, VaultError,
};

#[derive(Debug, Default)]
pub struct DesktopShell {
    active_vault: Option<Vault>,
    app_state_dir: Option<PathBuf>,
    known_vault_roots: Vec<PathBuf>,
}

impl DesktopShell {
    pub fn with_app_state_dir(app_state_dir: impl AsRef<Path>) -> Self {
        let app_state_dir = app_state_dir.as_ref().to_path_buf();
        let known_vault_roots = read_known_vault_roots(&app_state_dir).unwrap_or_default();
        Self {
            active_vault: None,
            app_state_dir: Some(app_state_dir),
            known_vault_roots,
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

    pub fn add_artwork_item(&self, item: AddArtworkItem) -> Result<SavedItem, DesktopShellError> {
        let vault = self
            .active_vault
            .as_ref()
            .ok_or(DesktopShellError::NoActiveVault)?;

        vault
            .add_artwork_item(item)
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
        let selected_item = match request.selected_item_id.as_deref() {
            Some(id) => Some(vault.item_details(id).map_err(DesktopShellError::Vault)?),
            None => None,
        };

        Ok(WorkbenchSnapshot {
            active_vault: ActiveVault::from(vault.root()),
            subvaults: vault.list_subvaults().map_err(DesktopShellError::Vault)?,
            collections: vault.list_collections().map_err(DesktopShellError::Vault)?,
            artwork_items: vault
                .browse_artwork_items(&request.home_subvault)
                .map_err(DesktopShellError::Vault)?,
            idea_sources: vault
                .browse_idea_sources()
                .map_err(DesktopShellError::Vault)?,
            review_queue: vault.review_queue().map_err(DesktopShellError::Vault)?,
            search_results,
            selected_item,
        })
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
        self.active_vault = Some(vault);
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
}

#[derive(Debug, Default)]
pub struct TauriCommandState {
    shell: DesktopShell,
}

impl TauriCommandState {
    pub fn create_vault(&mut self, root: String) -> Result<ActiveVaultView, DesktopShellError> {
        self.shell
            .create_vault(root)
            .map(ActiveVaultView::from)
            .map_err(DesktopShellError::Vault)
    }

    pub fn open_vault(&mut self, root: String) -> Result<ActiveVaultView, DesktopShellError> {
        self.shell
            .open_vault(root)
            .map(ActiveVaultView::from)
            .map_err(DesktopShellError::Vault)
    }

    pub fn import_paintings(
        &self,
        command: ImportPaintingsCommand,
    ) -> Result<Vec<SavedItemView>, DesktopShellError> {
        self.shell
            .import_paintings_folder(command.source_folder)
            .map(|items| items.into_iter().map(SavedItemView::from).collect())
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
            .workbench_snapshot(WorkbenchRequest {
                home_subvault: command.home_subvault,
                search_query: command.search_query,
                selected_item_id: command.selected_item_id,
            })
            .map(WorkbenchSnapshotView::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPaintingsCommand {
    pub source_folder: String,
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
    pub search_query: Option<String>,
    pub selected_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkGridItemView {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub primary_file: String,
    pub thumbnail_file: String,
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
            review_status: item.review_status().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeaSourceItemView {
    pub id: String,
    pub title: String,
    pub source_link: String,
    pub source_copy: Option<String>,
    pub review_status: String,
    pub reason: Option<String>,
}

impl From<&IdeaSourceListItem> for IdeaSourceItemView {
    fn from(item: &IdeaSourceListItem) -> Self {
        Self {
            id: item.saved_item().id().to_string(),
            title: item.title().to_string(),
            source_link: item.source_link().to_string(),
            source_copy: item.source_copy().map(path_string),
            review_status: item.review_status().to_string(),
            reason: item.reason().map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueItemView {
    pub id: String,
    pub home_subvault: String,
    pub item_type: String,
    pub title: String,
    pub review_status: String,
    pub reason: Option<String>,
}

impl From<&ReviewQueueItem> for ReviewQueueItemView {
    fn from(item: &ReviewQueueItem) -> Self {
        Self {
            id: item.saved_item().id().to_string(),
            home_subvault: item.home_subvault().to_string(),
            item_type: item.item_type().to_string(),
            title: item.title().to_string(),
            review_status: item.review_status().to_string(),
            reason: item.reason().map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultView {
    pub id: String,
    pub home_subvault: String,
}

impl From<&SearchResult> for SearchResultView {
    fn from(result: &SearchResult) -> Self {
        Self {
            id: result.saved_item().id().to_string(),
            home_subvault: result.saved_item().home_subvault().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDetailsView {
    pub id: String,
    pub home_subvault: String,
    pub item_folder: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub primary_file: String,
    pub review_status: String,
    pub tags: Vec<String>,
    pub collections: Vec<String>,
    pub saving_reason: Option<String>,
    pub source_link: Option<String>,
    pub summary: Option<String>,
    pub source_copy: Option<String>,
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
            tags: details.tags().into_iter().map(str::to_string).collect(),
            collections: details
                .collections()
                .into_iter()
                .map(str::to_string)
                .collect(),
            saving_reason: details.saving_reason().map(str::to_string),
            source_link: details.source_link().map(str::to_string),
            summary: details.summary().map(str::to_string),
            source_copy: details.source_copy().map(path_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchSnapshotView {
    pub active_vault: ActiveVaultView,
    pub subvaults: Vec<String>,
    pub collections: Vec<CollectionView>,
    pub artwork_items: Vec<ArtworkGridItemView>,
    pub idea_sources: Vec<IdeaSourceItemView>,
    pub review_queue: Vec<ReviewQueueItemView>,
    pub search_results: Vec<SearchResultView>,
    pub selected_item: Option<ItemDetailsView>,
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
        }
    }
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
    AppStateNotConfigured,
    MalformedProviderConfig(PathBuf),
    Io(io::Error),
    Vault(VaultError),
}

impl std::fmt::Display for DesktopShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoActiveVault => write!(f, "no active vault is open"),
            Self::AppStateNotConfigured => write!(f, "app state directory is not configured"),
            Self::MalformedProviderConfig(path) => {
                write!(f, "provider config is malformed: {}", path.display())
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
            Self::AppStateNotConfigured => None,
            Self::MalformedProviderConfig(_) => None,
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
