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
