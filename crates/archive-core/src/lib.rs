use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

const VAULT_CONFIG_FILE: &str = "vault.toml";
const SUBVAULTS_DIR: &str = "subvaults";
const COLLECTIONS_DIR: &str = "collections";
const HIDDEN_STATE_DIR: &str = ".gruenesgewolbe";
const METADATA_INDEX_FILE: &str = "metadata-index.tsv";
const AI_COST_LOG_FILE: &str = "ai-cost-log.tsv";
const ACTIVITY_LOG_FILE: &str = "activity-log.tsv";
const THUMBNAILS_DIR: &str = "thumbnails";
const TAG_REGISTRY_FILE: &str = "tag-registry.md";
const DEFAULT_VAULT_NAME: &str = "Personal Archive";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    pub fn create(root: impl AsRef<Path>) -> Result<Self, VaultError> {
        let root = root.as_ref();
        if root.exists() && fs::read_dir(root)?.next().is_some() {
            return Err(VaultError::NonEmptyRoot(root.to_path_buf()));
        }

        fs::create_dir_all(root)?;
        fs::create_dir_all(root.join(SUBVAULTS_DIR))?;
        fs::create_dir_all(root.join(COLLECTIONS_DIR))?;
        fs::create_dir_all(root.join(HIDDEN_STATE_DIR))?;
        fs::write(root.join(VAULT_CONFIG_FILE), default_config())?;

        Self::open(root)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, VaultError> {
        let root = root.as_ref();
        validate_vault_root(root)?;

        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn open_or_repair(root: impl AsRef<Path>) -> Result<VaultOpen, VaultError> {
        let root = root.as_ref();
        validate_vault_config(root)?;
        let directories = missing_repairable_directories(root)?;

        if directories.is_empty() {
            return Ok(VaultOpen::Opened(Self {
                root: root.to_path_buf(),
            }));
        }

        Ok(VaultOpen::RepairRequired(VaultRepairProposal {
            root: root.to_path_buf(),
            directories,
        }))
    }

    pub fn validate(root: impl AsRef<Path>) -> Result<(), VaultError> {
        validate_vault_root(root.as_ref())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list_subvaults(&self) -> Result<Vec<String>, VaultError> {
        let mut subvaults = Vec::new();
        for entry in fs::read_dir(self.root.join(SUBVAULTS_DIR))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            subvaults.push(entry.file_name().to_string_lossy().to_string());
        }
        subvaults.sort();
        Ok(subvaults)
    }

    pub fn list_collections(&self) -> Result<Vec<Collection>, VaultError> {
        let mut collections = Vec::new();
        for entry in fs::read_dir(self.root.join(COLLECTIONS_DIR))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            let text = fs::read_to_string(&path)?;
            collections.push(Collection {
                id: required_frontmatter_value(&path, &text, "id")?,
                name: required_frontmatter_value(&path, &text, "name")?,
            });
        }
        collections.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(collections)
    }

    pub fn add_artwork_item(&self, item: AddArtworkItem) -> Result<SavedItem, VaultError> {
        self.preserve_artwork_item(item, true, false)
            .map(|outcome| outcome.saved_item)
    }

    fn preserve_artwork_item(
        &self,
        item: AddArtworkItem,
        refresh_metadata_index: bool,
        suppress_fingerprint_candidates: bool,
    ) -> Result<PreservedArtwork, VaultError> {
        let file_name = item
            .source_file
            .file_name()
            .ok_or_else(|| VaultError::MissingFileName(item.source_file.clone()))?;
        if !item.source_file.is_file() {
            return Err(VaultError::MissingSourceFile(item.source_file));
        }

        let folder_name = artwork_folder_name(
            item.creator.as_deref(),
            item.year.as_deref(),
            item.title.as_str(),
        );
        let items_root = self
            .root
            .join(SUBVAULTS_DIR)
            .join(&item.home_subvault)
            .join("items");
        fs::create_dir_all(&items_root)?;

        let item_folder = unique_folder_path(&items_root, &folder_name);
        let files_dir = item_folder.join("files");
        fs::create_dir_all(&files_dir)?;

        let preserved_file = files_dir.join(file_name);
        fs::copy(&item.source_file, &preserved_file)?;

        let id = new_item_id();
        let file_fingerprint = file_fingerprint(&item.source_file)?;
        let duplicate_candidates = self.duplicate_candidates_for_artwork(
            (!suppress_fingerprint_candidates).then_some(file_fingerprint.as_str()),
            item.creator.as_deref(),
            item.year.as_deref(),
            Some(item.title.as_str()),
            Some(&item.source_file),
            None,
        )?;
        let primary_file = format!("files/{}", file_name.to_string_lossy());
        let record = artwork_record(
            &id,
            &item,
            &primary_file,
            &file_name.to_string_lossy(),
            &item.source_file,
            &imported_at(),
            &file_fingerprint,
            &duplicate_candidates,
        )?;
        fs::write(item_folder.join("record.md"), record)?;
        if refresh_metadata_index {
            self.rebuild_metadata_index()?;
        }

        Ok(PreservedArtwork {
            duplicate_candidate_count: duplicate_candidates.len(),
            saved_item: SavedItem {
                id,
                home_subvault: item.home_subvault,
                item_folder,
            },
        })
    }

    pub fn add_artwork_files(
        &self,
        source_files: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Vec<SavedItem>, VaultError> {
        self.add_artwork_files_with_metadata(source_files, ArtworkImportMetadata::default())
    }

    pub fn add_artwork_files_with_metadata(
        &self,
        source_files: impl IntoIterator<Item = PathBuf>,
        metadata: ArtworkImportMetadata,
    ) -> Result<Vec<SavedItem>, VaultError> {
        let source_files = source_files.into_iter().collect::<Vec<_>>();
        for source_file in &source_files {
            if !is_supported_image_file(source_file) {
                return Err(VaultError::UnsupportedImageFile(source_file.clone()));
            }
        }

        source_files
            .into_iter()
            .map(|source_file| self.add_artwork_item(inferred_artwork_item(source_file, &metadata)))
            .collect()
    }

    pub fn import_paintings_folder(
        &self,
        source_folder: impl AsRef<Path>,
    ) -> Result<Vec<SavedItem>, VaultError> {
        let source_folder = source_folder.as_ref();
        if !source_folder.is_dir() {
            return Err(VaultError::MissingImportFolder(source_folder.to_path_buf()));
        }

        let mut imported = Vec::new();
        for entry in fs::read_dir(source_folder)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            if !is_supported_image_file(&path) {
                continue;
            }

            imported.push(self.add_artwork_item(inferred_artwork_item(
                path,
                &ArtworkImportMetadata::default(),
            ))?);
        }

        self.append_activity_log(&format!(
            "import-paintings\t{}\t{}",
            source_folder.display(),
            imported.len()
        ))?;

        Ok(imported)
    }

    pub fn run_paintings_import<F>(
        &self,
        source_folder: impl AsRef<Path>,
        on_progress: F,
    ) -> Result<ImportRunSummary, VaultError>
    where
        F: FnMut(&ImportProgress) -> ImportRunAction,
    {
        self.run_paintings_import_with_metadata(
            source_folder,
            ArtworkImportMetadata::default(),
            on_progress,
        )
    }

    pub fn run_paintings_import_with_metadata<F>(
        &self,
        source_folder: impl AsRef<Path>,
        metadata: ArtworkImportMetadata,
        on_progress: F,
    ) -> Result<ImportRunSummary, VaultError>
    where
        F: FnMut(&ImportProgress) -> ImportRunAction,
    {
        self.run_paintings_import_with_options(
            source_folder,
            ImportRunOptions {
                metadata,
                exact_duplicate_policy: ExactDuplicatePolicy::Skip,
            },
            on_progress,
        )
    }

    pub fn run_paintings_import_with_options<F>(
        &self,
        source_folder: impl AsRef<Path>,
        options: ImportRunOptions,
        mut on_progress: F,
    ) -> Result<ImportRunSummary, VaultError>
    where
        F: FnMut(&ImportProgress) -> ImportRunAction,
    {
        let source_folder = source_folder.as_ref();
        if !source_folder.is_dir() {
            return Err(VaultError::MissingImportFolder(source_folder.to_path_buf()));
        }

        let mut source_files = Vec::new();
        let mut skipped_entries = Vec::new();
        let mut failed_entries = Vec::new();
        let mut maintenance_errors = Vec::new();
        discover_import_entries(
            source_folder,
            &mut source_files,
            &mut skipped_entries,
            &mut failed_entries,
        );
        source_files.sort();
        skipped_entries.sort_by(|left, right| left.path.cmp(&right.path));
        let mut imported_items = Vec::new();
        let mut duplicate_candidate_entries = Vec::new();
        let mut cancelled_files = Vec::new();
        let mut vault_problems = Vec::new();
        for failure in &failed_entries {
            if let Err(error) = self.append_import_file_failure(failure) {
                maintenance_errors.push(format!("activity-log: {error}"));
            }
        }
        for (processed, source_file) in source_files.iter().enumerate() {
            let progress = ImportProgress {
                processed,
                total: source_files.len(),
                current_file: source_file.clone(),
            };
            if on_progress(&progress) == ImportRunAction::Cancel {
                cancelled_files.extend_from_slice(&source_files[processed..]);
                break;
            }
            let duplicate_check = match self.exact_file_duplicate_check(source_file) {
                Ok(check) => check,
                Err(error) => {
                    let failure = ImportFailedEntry {
                        path: source_file.clone(),
                        error: error.to_string(),
                    };
                    if let Err(log_error) = self.append_import_file_failure(&failure) {
                        maintenance_errors.push(format!("activity-log: {log_error}"));
                    }
                    failed_entries.push(failure);
                    continue;
                }
            };
            vault_problems.extend(duplicate_check.vault_problems);
            let is_exact_duplicate = duplicate_check.existing_item_id.is_some();
            if options.exact_duplicate_policy == ExactDuplicatePolicy::Skip {
                if let Some(existing_item_id) = duplicate_check.existing_item_id {
                    skipped_entries.push(ImportSkippedEntry {
                        path: source_file.clone(),
                        reason: ImportSkipReason::ExactFileDuplicate { existing_item_id },
                    });
                    continue;
                }
            }
            match self.preserve_artwork_item(
                inferred_artwork_item(source_file.clone(), &options.metadata),
                false,
                is_exact_duplicate,
            ) {
                Ok(outcome) => {
                    if outcome.duplicate_candidate_count > 0 {
                        duplicate_candidate_entries.push(ImportDuplicateCandidateEntry {
                            path: source_file.clone(),
                            item_id: outcome.saved_item.id.clone(),
                            candidate_count: outcome.duplicate_candidate_count,
                        });
                    }
                    imported_items.push(outcome.saved_item);
                }
                Err(error) => {
                    let failure = ImportFailedEntry {
                        path: source_file.clone(),
                        error: error.to_string(),
                    };
                    if let Err(log_error) = self.append_import_file_failure(&failure) {
                        maintenance_errors.push(format!("activity-log: {log_error}"));
                    }
                    failed_entries.push(failure);
                }
            }
        }

        if let Err(error) = self.rebuild_metadata_index() {
            maintenance_errors.push(format!("metadata-index: {error}"));
        }
        let summary_event = format!(
            "import-run-{}\t{}\timported={}\tskipped={}\tduplicate-candidates={}\tcancelled={}\tfailed={}",
            if !cancelled_files.is_empty() {
                "cancelled"
            } else {
                "completed"
            },
            activity_log_field(&source_folder.display().to_string()),
            imported_items.len(),
            skipped_entries.len(),
            duplicate_candidate_entries.len(),
            cancelled_files.len(),
            failed_entries.len()
        );
        if let Err(error) = self.append_activity_log(&summary_event) {
            maintenance_errors.push(format!("activity-log: {error}"));
        }

        Ok(ImportRunSummary {
            imported_items,
            skipped_entries,
            failed_entries,
            duplicate_candidate_entries,
            cancelled_files,
            maintenance_errors,
            vault_problems,
        })
    }

    pub fn manual_fallback_capture(
        &self,
        capture: ManualFallbackCapture,
    ) -> Result<SavedItem, VaultError> {
        self.capture_idea_source(IdeaSourceCapture {
            source_link: capture.source_link,
            title: capture.title,
            saving_reason: capture.saving_reason,
            copied_text: capture.copied_text,
            copied_image: capture.copied_image,
            capture_method: "manual-fallback",
        })
    }

    pub fn capture_extracted_text(
        &self,
        capture: ExtractedTextCapture,
    ) -> Result<SavedItem, VaultError> {
        self.capture_idea_source(IdeaSourceCapture {
            source_link: capture.source_link,
            title: capture.title,
            saving_reason: capture.saving_reason,
            copied_text: Some(capture.cleaned_text),
            copied_image: None,
            capture_method: "extracted-text",
        })
    }

    pub fn capture_source_link(
        &self,
        capture: SourceLinkCapture,
        extractor: &dyn SourceExtractor,
    ) -> Result<SourceCaptureResult, VaultError> {
        match extractor.extract(SourceExtractionRequest {
            source_link: capture.source_link.clone(),
        }) {
            SourceExtraction::ExtractedText {
                title,
                cleaned_text,
            } => self
                .capture_extracted_text(ExtractedTextCapture {
                    source_link: capture.source_link,
                    title: title.unwrap_or(capture.title),
                    saving_reason: capture.saving_reason,
                    cleaned_text,
                })
                .map(SourceCaptureResult::Captured),
            SourceExtraction::NeedsManualFallback { reason } => Ok(
                SourceCaptureResult::NeedsManualFallback(ManualFallbackPrompt {
                    source_link: capture.source_link,
                    title: capture.title,
                    saving_reason: capture.saving_reason,
                    reason,
                }),
            ),
        }
    }

    fn capture_idea_source(&self, capture: IdeaSourceCapture) -> Result<SavedItem, VaultError> {
        let folder_name = readable_part(Some(&capture.title), "Untitled Capture");
        let items_root = self
            .root
            .join(SUBVAULTS_DIR)
            .join("Idea Sources")
            .join("items");
        fs::create_dir_all(&items_root)?;

        let item_folder = unique_folder_path(&items_root, &folder_name);
        fs::create_dir_all(&item_folder)?;

        let source_copy = if let Some(copied_text) = capture.copied_text.as_deref() {
            let source_copies = item_folder.join("source-copies");
            fs::create_dir_all(&source_copies)?;
            let cleaned_text = source_copies.join("cleaned-text.md");
            fs::write(&cleaned_text, format!("{copied_text}\n"))?;
            Some("source-copies/cleaned-text.md".to_string())
        } else {
            None
        };

        let primary_file = if let Some(copied_image) = capture.copied_image.as_ref() {
            let files_dir = item_folder.join("files");
            fs::create_dir_all(&files_dir)?;
            let file_name = readable_part(Some(&copied_image.file_name), "copied-image");
            fs::write(files_dir.join(&file_name), &copied_image.bytes)?;
            Some(format!("files/{file_name}"))
        } else {
            None
        };

        let duplicate_candidates = self.duplicate_candidates_for_artwork(
            None,
            None,
            None,
            Some(capture.title.as_str()),
            None,
            Some(capture.source_link.as_str()),
        )?;
        let id = new_item_id();
        let record = idea_source_record(
            &id,
            &capture,
            source_copy.as_deref(),
            primary_file.as_deref(),
            &duplicate_candidates,
        );
        fs::write(item_folder.join("record.md"), record)?;

        self.rebuild_metadata_index()?;
        self.append_activity_log(&format!(
            "capture\t{}\tIdea Sources\t{}",
            id, capture.capture_method
        ))?;

        Ok(SavedItem {
            id,
            home_subvault: "Idea Sources".to_string(),
            item_folder,
        })
    }

    pub fn rebuild_metadata_index(&self) -> Result<RebuiltMetadataIndex, VaultError> {
        let records = self.item_record_entries()?;
        let collection_search_text = self.collection_search_text_by_item_id()?;
        let hidden_state = self.root.join(HIDDEN_STATE_DIR);
        fs::create_dir_all(&hidden_state)?;

        let mut index = String::new();
        let mut indexed_items = 0;
        for record in records {
            let text = fs::read_to_string(&record.record_path)?;
            let id = frontmatter_value(&text, "id")
                .ok_or_else(|| VaultError::MalformedItemRecord(record.record_path.clone()))?;
            let home_subvault = frontmatter_value(&text, "home_subvault")
                .ok_or_else(|| VaultError::MalformedItemRecord(record.record_path.clone()))?;
            let mut searchable_source = text;
            if let Some(collection_text) = collection_search_text.get(&id) {
                searchable_source.push('\n');
                searchable_source.push_str(collection_text);
            }
            let searchable_text = normalize_search_text(&searchable_source);

            index.push_str(&escape_index_field(&id));
            index.push('\t');
            index.push_str(&escape_index_field(&home_subvault));
            index.push('\t');
            index.push_str(&escape_index_field(
                &record.item_folder.display().to_string(),
            ));
            index.push('\t');
            index.push_str(&escape_index_field(&searchable_text));
            index.push('\n');
            indexed_items += 1;
        }

        fs::write(hidden_state.join(METADATA_INDEX_FILE), index)?;
        self.append_activity_log(&format!("rebuild-metadata-index\t{indexed_items}"))?;

        Ok(RebuiltMetadataIndex { indexed_items })
    }

    pub fn search_metadata(&self, query: &str) -> Result<Vec<SearchResult>, VaultError> {
        let index_path = self.root.join(HIDDEN_STATE_DIR).join(METADATA_INDEX_FILE);
        if !index_path.is_file() {
            self.rebuild_metadata_index()?;
        }

        let query = normalize_search_text(query);
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let index = fs::read_to_string(index_path)?;
        let mut results = Vec::new();
        for line in index.lines() {
            let fields: Vec<_> = line.split('\t').map(unescape_index_field).collect();
            let [id, home_subvault, item_folder, searchable_text] = fields.as_slice() else {
                continue;
            };

            if !searchable_text.contains(&query) {
                continue;
            }

            results.push(SearchResult {
                saved_item: SavedItem {
                    id: id.to_string(),
                    home_subvault: home_subvault.to_string(),
                    item_folder: PathBuf::from(item_folder),
                },
            });
        }

        Ok(results)
    }

    pub fn browse_artwork_items(
        &self,
        home_subvault: &str,
    ) -> Result<Vec<ArtworkGridItem>, VaultError> {
        self.browse_artwork_items_sorted(home_subvault, ArtworkSort::Newest)
    }

    pub fn browse_artwork_items_sorted(
        &self,
        home_subvault: &str,
        sort: ArtworkSort,
    ) -> Result<Vec<ArtworkGridItem>, VaultError> {
        let mut items = Vec::new();
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "home_subvault").as_deref() != Some(home_subvault) {
                continue;
            }
            if frontmatter_value(&text, "item_type").as_deref() != Some("artwork") {
                continue;
            }

            let saved_item = saved_item_from_record(&record, &text)?;
            let primary_file = record.item_folder.join(required_frontmatter_value(
                &record.record_path,
                &text,
                "primary_file",
            )?);
            let thumbnail = self.cached_thumbnail_for(&saved_item, &primary_file)?;

            items.push(ArtworkGridItem {
                saved_item,
                title: required_frontmatter_value(&record.record_path, &text, "title")?,
                creator: required_frontmatter_value(&record.record_path, &text, "creator")?,
                year: required_frontmatter_value(&record.record_path, &text, "year")?,
                primary_file,
                thumbnail_file: thumbnail.path,
                thumbnail_is_placeholder: thumbnail.is_placeholder,
                added_at: frontmatter_value(&text, "imported_at")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_default(),
                review_status: required_frontmatter_value(
                    &record.record_path,
                    &text,
                    "review_status",
                )?,
            });
        }

        items.sort_by(|left, right| match sort {
            ArtworkSort::Newest => right
                .added_at
                .cmp(&left.added_at)
                .then_with(|| right.saved_item.id.cmp(&left.saved_item.id)),
            ArtworkSort::Oldest => left
                .added_at
                .cmp(&right.added_at)
                .then_with(|| left.saved_item.id.cmp(&right.saved_item.id)),
            ArtworkSort::Title => left
                .title
                .to_ascii_lowercase()
                .cmp(&right.title.to_ascii_lowercase())
                .then_with(|| left.saved_item.id.cmp(&right.saved_item.id)),
            ArtworkSort::Creator => left
                .creator
                .to_ascii_lowercase()
                .cmp(&right.creator.to_ascii_lowercase())
                .then_with(|| left.title.cmp(&right.title)),
            ArtworkSort::Year => left
                .year
                .cmp(&right.year)
                .then_with(|| left.title.cmp(&right.title)),
        });
        Ok(items)
    }

    pub fn browse_idea_sources(&self) -> Result<Vec<IdeaSourceListItem>, VaultError> {
        let mut items = Vec::new();
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "item_type").as_deref() != Some("idea") {
                continue;
            }

            items.push(IdeaSourceListItem {
                saved_item: saved_item_from_record(&record, &text)?,
                title: required_frontmatter_value(&record.record_path, &text, "title")?,
                source_link: required_frontmatter_value(&record.record_path, &text, "source_link")?,
                source_copy: frontmatter_value(&text, "source_copy")
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from),
                review_status: required_frontmatter_value(
                    &record.record_path,
                    &text,
                    "review_status",
                )?,
                reason: markdown_section(&text, "Saving Reason"),
            });
        }

        items.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.saved_item.id.cmp(&right.saved_item.id))
        });
        Ok(items)
    }

    pub fn review_queue(&self) -> Result<Vec<ReviewQueueItem>, VaultError> {
        let mut items = Vec::new();
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            let review_status =
                required_frontmatter_value(&record.record_path, &text, "review_status")?;
            if review_status != "needs-review" {
                continue;
            }

            items.push(ReviewQueueItem {
                saved_item: saved_item_from_record(&record, &text)?,
                item_type: required_frontmatter_value(&record.record_path, &text, "item_type")?,
                title: required_frontmatter_value(&record.record_path, &text, "title")?,
                review_status,
                reason: markdown_section(&text, "Saving Reason"),
            });
        }

        items.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.saved_item.id.cmp(&right.saved_item.id))
        });
        Ok(items)
    }

    pub fn item_details(&self, id: &str) -> Result<ItemDetails, VaultError> {
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(id) {
                continue;
            }
            let item_type = required_frontmatter_value(&record.record_path, &text, "item_type")?;
            let title = required_frontmatter_value(&record.record_path, &text, "title")?;
            let creator = required_frontmatter_value(&record.record_path, &text, "creator")?;
            let year = required_frontmatter_value(&record.record_path, &text, "year")?;

            return Ok(ItemDetails {
                saved_item: saved_item_from_record(&record, &text)?,
                title: title.clone(),
                creator: creator.clone(),
                year: year.clone(),
                primary_file: record.item_folder.join(required_frontmatter_value(
                    &record.record_path,
                    &text,
                    "primary_file",
                )?),
                review_status: required_frontmatter_value(
                    &record.record_path,
                    &text,
                    "review_status",
                )?,
                review_reasons: frontmatter_list(&text, "review_reasons"),
                tags: frontmatter_list(&text, "tags"),
                collections: self.collection_names_for_item(id, &text)?,
                item_links: item_links(&text),
                duplicate_candidates: duplicate_candidates(&text),
                saving_reason: markdown_section(&text, "Saving Reason"),
                source_link: frontmatter_value(&text, "source_link"),
                summary: markdown_section(&text, "Summary"),
                source_copy: frontmatter_value(&text, "source_copy").and_then(|path| {
                    if path.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(path))
                    }
                }),
                metadata_provenance: metadata_provenance(&text),
                metadata_suggestions: metadata_suggestions(&text),
                better_file_candidates: better_file_candidates(&text),
                folder_rename_suggestion: folder_rename_suggestion(
                    &record.item_folder,
                    &item_type,
                    &creator,
                    &year,
                    &title,
                ),
                import_original_filename: frontmatter_value(&text, "import_original_filename"),
                import_source_path: frontmatter_value(&text, "import_source_path")
                    .map(PathBuf::from),
            });
        }

        Err(VaultError::SavedItemNotFound(id.to_string()))
    }

    pub fn update_item_record(&self, update: UpdateItemRecord) -> Result<ItemDetails, VaultError> {
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(update.id.as_str()) {
                continue;
            }

            let mut frontmatter_updates = Vec::new();
            if let Some(title) = update.title.as_deref() {
                frontmatter_updates.push(("title", serde_yaml::Value::String(title.to_string())));
            }
            if let Some(creator) = update.creator.as_deref() {
                frontmatter_updates
                    .push(("creator", serde_yaml::Value::String(creator.to_string())));
            }
            if let Some(year) = update.year.as_deref() {
                frontmatter_updates.push(("year", serde_yaml::Value::String(year.to_string())));
            }
            if let Some(review_status) = update.review_status.as_deref() {
                frontmatter_updates.push((
                    "review_status",
                    serde_yaml::Value::String(review_status.to_string()),
                ));
            }
            let mut updated = update_frontmatter_values(&text, &frontmatter_updates)?;
            if let Some(saving_reason) = update.saving_reason.as_deref() {
                updated = replace_markdown_section(&updated, "Saving Reason", saving_reason);
            }

            fs::write(&record.record_path, updated)?;
            self.rebuild_metadata_index()?;
            return self.item_details(&update.id);
        }

        Err(VaultError::SavedItemNotFound(update.id))
    }

    pub fn enrich_idea_with_ai(
        &self,
        id: &str,
        budget_mode: AiBudgetMode,
        provider: &dyn AiProvider,
    ) -> Result<AiEnrichmentResult, VaultError> {
        if budget_mode == AiBudgetMode::Off {
            self.open_saved_item(id)?;
            return Ok(AiEnrichmentResult {
                accepted_summary: None,
                accepted_tags: Vec::new(),
                accepted_metadata: Vec::new(),
                staged_suggestions: Vec::new(),
                better_file_candidates: Vec::new(),
                estimated_cost_cents: 0,
            });
        }

        let details = self.item_details(id)?;
        let cleaned_text = details
            .source_copy()
            .map(|path| fs::read_to_string(details.item_folder().join(path)))
            .transpose()?
            .map(|text| text.trim_end_matches('\n').to_string());

        let response = provider.enrich(AiProviderRequest {
            item_id: id.to_string(),
            budget_mode,
            cleaned_text,
            raw_html: None,
            image_bytes: None,
            related_item_records: Vec::new(),
        });

        self.apply_ai_enrichment_response(id, budget_mode, response)
    }

    pub fn suggest_artwork_metadata_with_ai(
        &self,
        id: &str,
        budget_mode: AiBudgetMode,
        provider: &dyn AiProvider,
    ) -> Result<AiEnrichmentResult, VaultError> {
        if budget_mode == AiBudgetMode::Off {
            self.open_saved_item(id)?;
            return Ok(AiEnrichmentResult {
                accepted_summary: None,
                accepted_tags: Vec::new(),
                accepted_metadata: Vec::new(),
                staged_suggestions: Vec::new(),
                better_file_candidates: Vec::new(),
                estimated_cost_cents: 0,
            });
        }

        let details = self.item_details(id)?;
        let response = provider.enrich(AiProviderRequest {
            item_id: id.to_string(),
            budget_mode,
            cleaned_text: None,
            raw_html: None,
            image_bytes: Some(fs::read(details.primary_file())?),
            related_item_records: Vec::new(),
        });

        self.apply_ai_enrichment_response(id, budget_mode, response)
    }

    pub fn upsert_tag(&self, tag: TagDefinition) -> Result<(), VaultError> {
        let mut tags = self.read_tag_registry()?;
        tags.retain(|existing| existing.name != tag.name);
        tags.push(tag);
        tags.sort_by(|left, right| left.name.cmp(&right.name));
        self.write_tag_registry(&tags)
    }

    pub fn add_tags_to_item(&self, id: &str, tags: Vec<String>) -> Result<ItemDetails, VaultError> {
        let registry = self.read_tag_registry()?;
        let canonical_tags = tags
            .into_iter()
            .map(|tag| canonical_tag(&registry, &tag))
            .collect::<Vec<_>>();

        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(id) {
                continue;
            }

            let mut existing_tags = frontmatter_list(&text, "tags");
            for tag in canonical_tags {
                if !existing_tags.contains(&tag) {
                    existing_tags.push(tag);
                }
            }
            existing_tags.sort();

            let updated = update_frontmatter_values(
                &text,
                &[(
                    "tags",
                    serde_yaml::Value::Sequence(
                        existing_tags
                            .into_iter()
                            .map(serde_yaml::Value::String)
                            .collect(),
                    ),
                )],
            )?;
            fs::write(&record.record_path, updated)?;
            self.rebuild_metadata_index()?;
            return self.item_details(id);
        }

        Err(VaultError::SavedItemNotFound(id.to_string()))
    }

    pub fn create_collection(
        &self,
        collection: CollectionDefinition,
    ) -> Result<Collection, VaultError> {
        let id = slugify(&collection.name);
        let path = self.root.join(COLLECTIONS_DIR).join(format!("{id}.md"));
        let record = collection_record(&id, &collection, &[]);
        fs::write(path, record)?;

        Ok(Collection {
            id,
            name: collection.name,
        })
    }

    pub fn add_item_to_collection(
        &self,
        collection_id: &str,
        item_id: &str,
    ) -> Result<ItemDetails, VaultError> {
        let collection_path = self
            .root
            .join(COLLECTIONS_DIR)
            .join(format!("{collection_id}.md"));
        if !collection_path.is_file() {
            return Err(VaultError::CollectionNotFound(collection_id.to_string()));
        }

        let collection_text = fs::read_to_string(&collection_path)?;
        let collection_name =
            required_frontmatter_value(&collection_path, &collection_text, "name")?;
        let mut item_ids = collection_items(&collection_text);
        if !item_ids.iter().any(|id| id == item_id) {
            item_ids.push(item_id.to_string());
        }
        fs::write(
            &collection_path,
            replace_collection_items(&collection_text, &item_ids),
        )?;

        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(item_id) {
                continue;
            }

            let mut collections = frontmatter_list(&text, "collections");
            if !collections.iter().any(|name| name == &collection_name) {
                collections.push(collection_name);
            }
            collections.sort();
            let updated = update_frontmatter_values(
                &text,
                &[(
                    "collections",
                    serde_yaml::Value::Sequence(
                        collections
                            .into_iter()
                            .map(serde_yaml::Value::String)
                            .collect(),
                    ),
                )],
            )?;
            fs::write(&record.record_path, updated)?;
            self.rebuild_metadata_index()?;
            return self.item_details(item_id);
        }

        Err(VaultError::SavedItemNotFound(item_id.to_string()))
    }

    pub fn add_item_link(
        &self,
        item_id: &str,
        link: ItemLinkDefinition,
    ) -> Result<ItemDetails, VaultError> {
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(item_id) {
                continue;
            }

            let line = item_link_line(&link);
            let mut lines = markdown_list_section(&text, "Item Links");
            if !lines.iter().any(|existing| existing == &line) {
                lines.push(line);
            }
            let updated = replace_or_append_markdown_list_section(&text, "Item Links", &lines);
            fs::write(&record.record_path, updated)?;
            self.rebuild_metadata_index()?;
            return self.item_details(item_id);
        }

        Err(VaultError::SavedItemNotFound(item_id.to_string()))
    }

    pub fn open_saved_item(&self, id: &str) -> Result<SavedItem, VaultError> {
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(id) {
                continue;
            }

            let home_subvault = frontmatter_value(&text, "home_subvault")
                .ok_or_else(|| VaultError::MalformedItemRecord(record.record_path.clone()))?;

            return Ok(SavedItem {
                id: id.to_string(),
                home_subvault,
                item_folder: record.item_folder,
            });
        }

        Err(VaultError::SavedItemNotFound(id.to_string()))
    }

    pub fn record_error_event(
        &self,
        operation: &str,
        target: &str,
        message: &str,
    ) -> Result<(), VaultError> {
        self.append_activity_log(&format!(
            "error\t{}\t{}\t{}",
            activity_log_field(operation),
            activity_log_field(target),
            activity_log_field(message)
        ))
    }

    fn item_record_entries(&self) -> Result<Vec<ItemRecordEntry>, VaultError> {
        let mut records = Vec::new();
        let subvaults_root = self.root.join(SUBVAULTS_DIR);
        for subvault in fs::read_dir(subvaults_root)? {
            let subvault = subvault?;
            if !subvault.file_type()?.is_dir() {
                continue;
            }

            let items_root = subvault.path().join("items");
            if !items_root.is_dir() {
                continue;
            }

            for item_folder in fs::read_dir(items_root)? {
                let item_folder = item_folder?;
                if !item_folder.file_type()?.is_dir() {
                    continue;
                }

                let record_path = item_folder.path().join("record.md");
                if !record_path.is_file() {
                    continue;
                }

                records.push(ItemRecordEntry {
                    record_path,
                    item_folder: item_folder.path(),
                });
            }
        }

        records.sort_by(|left, right| left.item_folder.cmp(&right.item_folder));
        Ok(records)
    }

    fn collection_search_text_by_item_id(&self) -> Result<HashMap<String, String>, VaultError> {
        let mut text_by_item_id = HashMap::new();
        let collections_root = self.root.join(COLLECTIONS_DIR);
        if !collections_root.is_dir() {
            return Ok(text_by_item_id);
        }

        for entry in fs::read_dir(collections_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let collection_text = fs::read_to_string(entry.path())?;
            for item_id in collection_items(&collection_text) {
                let item_text = text_by_item_id.entry(item_id).or_insert_with(String::new);
                if !item_text.is_empty() {
                    item_text.push('\n');
                }
                item_text.push_str(&collection_text);
            }
        }

        Ok(text_by_item_id)
    }

    fn collection_names_for_item(
        &self,
        item_id: &str,
        item_record: &str,
    ) -> Result<Vec<String>, VaultError> {
        let mut collections = frontmatter_list(item_record, "collections");
        let collections_root = self.root.join(COLLECTIONS_DIR);
        if !collections_root.is_dir() {
            collections.sort();
            return Ok(collections);
        }

        for entry in fs::read_dir(collections_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            let collection_text = fs::read_to_string(&path)?;
            if !collection_items(&collection_text)
                .iter()
                .any(|id| id == item_id)
            {
                continue;
            }

            let name = required_frontmatter_value(&path, &collection_text, "name")?;
            if !collections.iter().any(|collection| collection == &name) {
                collections.push(name);
            }
        }

        collections.sort();
        Ok(collections)
    }

    fn duplicate_candidates_for_artwork(
        &self,
        file_fingerprint: Option<&str>,
        creator: Option<&str>,
        year: Option<&str>,
        title: Option<&str>,
        source_path: Option<&Path>,
        source_link: Option<&str>,
    ) -> Result<Vec<DuplicateCandidate>, VaultError> {
        let mut candidates = Vec::new();
        for record in self.item_record_entries()? {
            let Ok(text) = fs::read_to_string(&record.record_path) else {
                continue;
            };
            let Some(id) = frontmatter_value(&text, "id") else {
                continue;
            };

            if let Some(file_fingerprint) = file_fingerprint {
                if frontmatter_value(&text, "file_fingerprint").as_deref() == Some(file_fingerprint)
                {
                    push_duplicate_candidate(&mut candidates, id.clone(), "file-fingerprint");
                }
            }

            if let Some(source_path) = source_path {
                if frontmatter_value(&text, "import_source_path")
                    .map(PathBuf::from)
                    .as_deref()
                    == Some(source_path)
                {
                    push_duplicate_candidate(&mut candidates, id.clone(), "import-provenance");
                }
            }

            if let Some(source_link) = source_link {
                if frontmatter_value(&text, "source_link").as_deref() == Some(source_link) {
                    push_duplicate_candidate(&mut candidates, id.clone(), "source-link");
                }
            }

            if let (Some(creator), Some(year), Some(title)) = (creator, year, title) {
                if frontmatter_value(&text, "creator").as_deref() == Some(creator)
                    && frontmatter_value(&text, "year").as_deref() == Some(year)
                    && frontmatter_value(&text, "title").as_deref() == Some(title)
                {
                    push_duplicate_candidate(&mut candidates, id, "descriptive-metadata");
                }
            }
        }

        Ok(candidates)
    }

    fn exact_file_duplicate_check(
        &self,
        source_file: &Path,
    ) -> Result<ExactDuplicateCheck, VaultError> {
        let incoming_fingerprint = file_fingerprint(source_file)?;
        let incoming_bytes = fs::read(source_file)?;
        let mut vault_problems = Vec::new();
        for record in self.item_record_entries()? {
            let text = match fs::read_to_string(&record.record_path) {
                Ok(text) => text,
                Err(error) => {
                    vault_problems.push(ImportVaultProblem {
                        path: record.record_path.clone(),
                        error: error.to_string(),
                    });
                    continue;
                }
            };
            if frontmatter_mapping(&text).is_none() {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "malformed item record frontmatter".to_string(),
                });
                continue;
            }
            if frontmatter_value(&text, "file_fingerprint").as_deref()
                != Some(incoming_fingerprint.as_str())
            {
                continue;
            }
            let Some(id) = frontmatter_value(&text, "id") else {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "missing item id".to_string(),
                });
                continue;
            };
            let Some(original_filename) = frontmatter_value(&text, "import_original_filename")
            else {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "missing preserved-file provenance".to_string(),
                });
                continue;
            };
            let original_path = Path::new(&original_filename);
            if original_path.file_name() != Some(original_path.as_os_str()) {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "invalid preserved-file provenance".to_string(),
                });
                continue;
            }
            let preserved_path = record.item_folder.join("files").join(original_path);
            match fs::read(&preserved_path) {
                Ok(existing_bytes) if existing_bytes == incoming_bytes => {
                    return Ok(ExactDuplicateCheck {
                        existing_item_id: Some(id),
                        vault_problems,
                    });
                }
                Ok(_) => {}
                Err(error) => vault_problems.push(ImportVaultProblem {
                    path: preserved_path,
                    error: error.to_string(),
                }),
            }
        }
        Ok(ExactDuplicateCheck {
            existing_item_id: None,
            vault_problems,
        })
    }

    fn append_import_file_failure(&self, failure: &ImportFailedEntry) -> Result<(), VaultError> {
        self.append_activity_log(&format!(
            "import-file-failed\t{}\t{}",
            activity_log_field(&failure.path.display().to_string()),
            activity_log_field(&failure.error)
        ))
    }

    fn read_tag_registry(&self) -> Result<Vec<TagDefinition>, VaultError> {
        let path = self.root.join(TAG_REGISTRY_FILE);
        if !path.is_file() {
            return Ok(Vec::new());
        }

        parse_tag_registry(&fs::read_to_string(path)?)
    }

    fn write_tag_registry(&self, tags: &[TagDefinition]) -> Result<(), VaultError> {
        let mut registry = "# Tag Registry\n".to_string();
        for tag in tags {
            registry.push('\n');
            registry.push_str(&format!("## {}\n\n", tag.name));
            registry.push_str("Aliases: ");
            registry.push_str(&tag.aliases.join(", "));
            registry.push_str("\n\n");
            if let Some(meaning) = tag.meaning.as_deref() {
                registry.push_str(meaning);
                registry.push('\n');
            }
        }

        fs::write(self.root.join(TAG_REGISTRY_FILE), registry)?;
        Ok(())
    }

    fn cached_thumbnail_for(
        &self,
        saved_item: &SavedItem,
        primary_file: &Path,
    ) -> Result<ThumbnailPreview, VaultError> {
        let thumbnails_dir = self.root.join(HIDDEN_STATE_DIR).join(THUMBNAILS_DIR);
        fs::create_dir_all(&thumbnails_dir)?;
        let thumbnail_file = thumbnails_dir.join(format!("{}.png", saved_item.id()));
        if thumbnail_file.is_file() {
            return Ok(ThumbnailPreview {
                path: thumbnail_file,
                is_placeholder: false,
            });
        }
        let placeholder_file = thumbnails_dir.join(format!("{}.placeholder.png", saved_item.id()));
        if placeholder_file.is_file() {
            return Ok(ThumbnailPreview {
                path: placeholder_file,
                is_placeholder: true,
            });
        }

        let decoded = image::io::Reader::open(primary_file)
            .map_err(|error| error.to_string())
            .and_then(|reader| {
                reader
                    .with_guessed_format()
                    .map_err(|error| error.to_string())
            })
            .and_then(|reader| reader.decode().map_err(|error| error.to_string()));
        match decoded {
            Ok(image) => {
                image
                    .thumbnail(480, 480)
                    .save_with_format(&thumbnail_file, image::ImageFormat::Png)
                    .map_err(|error| VaultError::PreviewGeneration(error.to_string()))?;
                Ok(ThumbnailPreview {
                    path: thumbnail_file,
                    is_placeholder: false,
                })
            }
            Err(reason) => {
                write_thumbnail_placeholder(&placeholder_file)?;
                self.record_thumbnail_preview_failure(saved_item, primary_file, &reason)?;
                Ok(ThumbnailPreview {
                    path: placeholder_file,
                    is_placeholder: true,
                })
            }
        }
    }

    fn record_thumbnail_preview_failure(
        &self,
        saved_item: &SavedItem,
        primary_file: &Path,
        reason: &str,
    ) -> Result<(), VaultError> {
        let record_path = saved_item.item_folder.join("record.md");
        let record = fs::read_to_string(&record_path)?;
        let review_reason = format!("thumbnail-preview-unavailable | {reason}");
        if frontmatter_list(&record, "review_reasons")
            .iter()
            .any(|existing| existing.starts_with("thumbnail-preview-unavailable"))
        {
            return Ok(());
        }

        let mut review_reasons = frontmatter_list(&record, "review_reasons");
        review_reasons.push(review_reason);
        let updated = update_frontmatter_values(
            &record,
            &[
                (
                    "review_reasons",
                    serde_yaml::Value::Sequence(
                        review_reasons
                            .into_iter()
                            .map(serde_yaml::Value::String)
                            .collect(),
                    ),
                ),
                (
                    "review_status",
                    serde_yaml::Value::String("needs-review".to_string()),
                ),
            ],
        )?;
        fs::write(record_path, updated)?;
        self.append_activity_log(&format!(
            "thumbnail-preview-failed\t{}\t{}\t{}",
            saved_item.id(),
            activity_log_field(&primary_file.display().to_string()),
            activity_log_field(reason)
        ))
    }

    fn apply_ai_enrichment_response(
        &self,
        id: &str,
        budget_mode: AiBudgetMode,
        response: AiProviderResponse,
    ) -> Result<AiEnrichmentResult, VaultError> {
        let registry = self.read_tag_registry()?;
        let mut accepted_tags = Vec::new();
        for tag in response.tags {
            let canonical = canonical_tag(&registry, &tag);
            if !accepted_tags.contains(&canonical) {
                accepted_tags.push(canonical);
            }
        }
        accepted_tags.sort();
        let accepted_summary = response.summary;
        let mut incoming_suggestions = Some(response.suggestions);
        let better_file_candidates = response.better_file_candidates;
        let estimated_cost_cents = response.estimated_cost_cents;

        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(id) {
                continue;
            }

            let mut updated = text;
            let (accepted_metadata, staged_suggestions) = split_metadata_suggestions(
                &updated,
                incoming_suggestions.take().unwrap_or_default(),
            );
            if !accepted_tags.is_empty() {
                let mut tags = frontmatter_list(&updated, "tags");
                for tag in &accepted_tags {
                    if !tags.contains(tag) {
                        tags.push(tag.clone());
                    }
                }
                tags.sort();
                updated = update_frontmatter_values(
                    &updated,
                    &[(
                        "tags",
                        serde_yaml::Value::Sequence(
                            tags.into_iter().map(serde_yaml::Value::String).collect(),
                        ),
                    )],
                )?;
            }
            if let Some(summary) = accepted_summary.as_deref() {
                updated = replace_or_append_markdown_section(&updated, "Summary", summary);
            }
            if !accepted_metadata.is_empty() {
                for suggestion in &accepted_metadata {
                    updated = update_frontmatter_values(
                        &updated,
                        &[(
                            suggestion.field(),
                            serde_yaml::Value::String(suggestion.suggested_value().to_string()),
                        )],
                    )?;
                }
                let provenance_lines = accepted_metadata
                    .iter()
                    .map(metadata_suggestion_line)
                    .collect::<Vec<_>>();
                updated = replace_or_append_markdown_list_section(
                    &updated,
                    "Metadata Provenance",
                    &provenance_lines,
                );
            }
            if !staged_suggestions.is_empty() {
                let suggestion_lines = staged_suggestions
                    .iter()
                    .map(metadata_suggestion_line)
                    .collect::<Vec<_>>();
                updated = replace_or_append_markdown_list_section(
                    &updated,
                    "Metadata Suggestions",
                    &suggestion_lines,
                );
                updated = update_frontmatter_values(
                    &updated,
                    &[(
                        "review_status",
                        serde_yaml::Value::String("needs-review".to_string()),
                    )],
                )?;
            }
            if !better_file_candidates.is_empty() {
                let candidate_lines = better_file_candidates
                    .iter()
                    .map(better_file_candidate_line)
                    .collect::<Vec<_>>();
                updated = replace_or_append_markdown_list_section(
                    &updated,
                    "Better File Candidates",
                    &candidate_lines,
                );
                updated = update_frontmatter_values(
                    &updated,
                    &[(
                        "review_status",
                        serde_yaml::Value::String("needs-review".to_string()),
                    )],
                )?;
            }

            fs::write(&record.record_path, updated)?;
            self.append_ai_cost_log(id, budget_mode, estimated_cost_cents)?;
            self.append_activity_log(&format!(
                "ai-cost\t{}\t{}\t{}",
                id,
                budget_mode.as_log_value(),
                estimated_cost_cents
            ))?;
            self.append_activity_log(&format!(
                "enrichment\t{}\t{}\t{}\t{}",
                id,
                budget_mode.as_log_value(),
                accepted_tags.len(),
                staged_suggestions.len()
            ))?;
            self.rebuild_metadata_index()?;
            return Ok(AiEnrichmentResult {
                accepted_summary,
                accepted_tags,
                accepted_metadata,
                staged_suggestions,
                better_file_candidates,
                estimated_cost_cents,
            });
        }

        Err(VaultError::SavedItemNotFound(id.to_string()))
    }

    fn append_ai_cost_log(
        &self,
        item_id: &str,
        budget_mode: AiBudgetMode,
        estimated_cost_cents: u32,
    ) -> Result<(), VaultError> {
        let hidden_state = self.root.join(HIDDEN_STATE_DIR);
        fs::create_dir_all(&hidden_state)?;
        let path = hidden_state.join(AI_COST_LOG_FILE);
        let mut existing = if path.is_file() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };

        existing.push_str(&format!(
            "{}\tai-enrichment\t{}\t{}\t{}\n",
            imported_at(),
            item_id,
            budget_mode.as_log_value(),
            estimated_cost_cents
        ));
        fs::write(path, existing)?;
        Ok(())
    }

    fn append_activity_log(&self, event: &str) -> Result<(), VaultError> {
        let hidden_state = self.root.join(HIDDEN_STATE_DIR);
        fs::create_dir_all(&hidden_state)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(hidden_state.join(ACTIVITY_LOG_FILE))?;
        writeln!(file, "{}\t{event}", imported_at())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultOpen {
    Opened(Vault),
    RepairRequired(VaultRepairProposal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRepairProposal {
    root: PathBuf,
    directories: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThumbnailPreview {
    path: PathBuf,
    is_placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreservedArtwork {
    saved_item: SavedItem,
    duplicate_candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactDuplicateCheck {
    existing_item_id: Option<String>,
    vault_problems: Vec<ImportVaultProblem>,
}

impl VaultRepairProposal {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    pub fn confirm(self) -> Result<Vault, VaultError> {
        validate_vault_config(&self.root)?;
        let current_directories = missing_repairable_directories(&self.root)?;
        if current_directories != self.directories {
            return Err(VaultError::RepairProposalChanged(self.root));
        }
        for directory in self.directories {
            fs::create_dir(&directory)?;
        }
        Vault::open(self.root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkGridItem {
    saved_item: SavedItem,
    title: String,
    creator: String,
    year: String,
    primary_file: PathBuf,
    thumbnail_file: PathBuf,
    thumbnail_is_placeholder: bool,
    added_at: u64,
    review_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkSort {
    Newest,
    Oldest,
    Title,
    Creator,
    Year,
}

impl ArtworkGridItem {
    pub fn saved_item(&self) -> &SavedItem {
        &self.saved_item
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn creator(&self) -> &str {
        &self.creator
    }

    pub fn year(&self) -> &str {
        &self.year
    }

    pub fn primary_file(&self) -> &Path {
        &self.primary_file
    }

    pub fn thumbnail_file(&self) -> &Path {
        &self.thumbnail_file
    }

    pub fn thumbnail_is_placeholder(&self) -> bool {
        self.thumbnail_is_placeholder
    }

    pub fn review_status(&self) -> &str {
        &self.review_status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeaSourceListItem {
    saved_item: SavedItem,
    title: String,
    source_link: String,
    source_copy: Option<PathBuf>,
    review_status: String,
    reason: Option<String>,
}

impl IdeaSourceListItem {
    pub fn saved_item(&self) -> &SavedItem {
        &self.saved_item
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn source_link(&self) -> &str {
        &self.source_link
    }

    pub fn source_copy(&self) -> Option<&Path> {
        self.source_copy.as_deref()
    }

    pub fn review_status(&self) -> &str {
        &self.review_status
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueItem {
    saved_item: SavedItem,
    item_type: String,
    title: String,
    review_status: String,
    reason: Option<String>,
}

impl ReviewQueueItem {
    pub fn saved_item(&self) -> &SavedItem {
        &self.saved_item
    }

    pub fn home_subvault(&self) -> &str {
        self.saved_item.home_subvault()
    }

    pub fn item_type(&self) -> &str {
        &self.item_type
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn review_status(&self) -> &str {
        &self.review_status
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDetails {
    saved_item: SavedItem,
    title: String,
    creator: String,
    year: String,
    primary_file: PathBuf,
    review_status: String,
    review_reasons: Vec<String>,
    tags: Vec<String>,
    collections: Vec<String>,
    item_links: Vec<ItemLink>,
    duplicate_candidates: Vec<DuplicateCandidate>,
    saving_reason: Option<String>,
    source_link: Option<String>,
    summary: Option<String>,
    source_copy: Option<PathBuf>,
    metadata_provenance: Vec<AiMetadataSuggestion>,
    metadata_suggestions: Vec<AiMetadataSuggestion>,
    better_file_candidates: Vec<BetterFileCandidate>,
    folder_rename_suggestion: Option<String>,
    import_original_filename: Option<String>,
    import_source_path: Option<PathBuf>,
}

impl ItemDetails {
    pub fn id(&self) -> &str {
        self.saved_item.id()
    }

    pub fn home_subvault(&self) -> &str {
        self.saved_item.home_subvault()
    }

    pub fn item_folder(&self) -> &Path {
        self.saved_item.item_folder()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn creator(&self) -> &str {
        &self.creator
    }

    pub fn year(&self) -> &str {
        &self.year
    }

    pub fn primary_file(&self) -> &Path {
        &self.primary_file
    }

    pub fn review_status(&self) -> &str {
        &self.review_status
    }

    pub fn review_reasons(&self) -> &[String] {
        &self.review_reasons
    }

    pub fn tags(&self) -> Vec<&str> {
        self.tags.iter().map(String::as_str).collect()
    }

    pub fn collections(&self) -> Vec<&str> {
        self.collections.iter().map(String::as_str).collect()
    }

    pub fn item_links(&self) -> &[ItemLink] {
        &self.item_links
    }

    pub fn duplicate_candidates(&self) -> &[DuplicateCandidate] {
        &self.duplicate_candidates
    }

    pub fn saving_reason(&self) -> Option<&str> {
        self.saving_reason.as_deref()
    }

    pub fn source_link(&self) -> Option<&str> {
        self.source_link.as_deref()
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn source_copy(&self) -> Option<&Path> {
        self.source_copy.as_deref()
    }

    pub fn metadata_provenance(&self) -> &[AiMetadataSuggestion] {
        &self.metadata_provenance
    }

    pub fn metadata_suggestions(&self) -> &[AiMetadataSuggestion] {
        &self.metadata_suggestions
    }

    pub fn better_file_candidates(&self) -> &[BetterFileCandidate] {
        &self.better_file_candidates
    }

    pub fn folder_rename_suggestion(&self) -> Option<&str> {
        self.folder_rename_suggestion.as_deref()
    }

    pub fn import_original_filename(&self) -> Option<&str> {
        self.import_original_filename.as_deref()
    }

    pub fn import_source_path(&self) -> Option<&Path> {
        self.import_source_path.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddArtworkItem {
    pub source_file: PathBuf,
    pub home_subvault: String,
    pub creator: Option<String>,
    pub year: Option<String>,
    pub title: String,
    pub saving_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtworkImportMetadata {
    pub creator: Option<String>,
    pub year: Option<String>,
    pub saving_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExactDuplicatePolicy {
    #[default]
    Skip,
    ImportAnyway,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportRunOptions {
    pub metadata: ArtworkImportMetadata,
    pub exact_duplicate_policy: ExactDuplicatePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportRunAction {
    Continue,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProgress {
    processed: usize,
    total: usize,
    current_file: PathBuf,
}

impl ImportProgress {
    pub fn processed(&self) -> usize {
        self.processed
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn current_file(&self) -> &Path {
        &self.current_file
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRunSummary {
    imported_items: Vec<SavedItem>,
    skipped_entries: Vec<ImportSkippedEntry>,
    failed_entries: Vec<ImportFailedEntry>,
    duplicate_candidate_entries: Vec<ImportDuplicateCandidateEntry>,
    cancelled_files: Vec<PathBuf>,
    maintenance_errors: Vec<String>,
    vault_problems: Vec<ImportVaultProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSkippedEntry {
    path: PathBuf,
    reason: ImportSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSkipReason {
    SymbolicLink,
    UnsupportedFile,
    ExactFileDuplicate { existing_item_id: String },
}

impl ImportSkipReason {
    fn code(&self) -> &'static str {
        match self {
            Self::SymbolicLink => "symbolic-link",
            Self::UnsupportedFile => "unsupported-file",
            Self::ExactFileDuplicate { .. } => "exact-file-duplicate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportVaultProblem {
    path: PathBuf,
    error: String,
}

impl ImportVaultProblem {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailedEntry {
    path: PathBuf,
    error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDuplicateCandidateEntry {
    path: PathBuf,
    item_id: String,
    candidate_count: usize,
}

impl ImportDuplicateCandidateEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}

impl ImportFailedEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

impl ImportSkippedEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reason(&self) -> &str {
        self.reason.code()
    }

    pub fn existing_item_id(&self) -> Option<&str> {
        match &self.reason {
            ImportSkipReason::ExactFileDuplicate { existing_item_id } => Some(existing_item_id),
            _ => None,
        }
    }
}

impl ImportRunSummary {
    pub fn imported_count(&self) -> usize {
        self.imported_items.len()
    }

    pub fn failed_count(&self) -> usize {
        self.failed_entries.len()
    }

    pub fn imported_items(&self) -> &[SavedItem] {
        &self.imported_items
    }

    pub fn skipped_count(&self) -> usize {
        self.skipped_entries.len()
    }

    pub fn skipped_entries(&self) -> &[ImportSkippedEntry] {
        &self.skipped_entries
    }

    pub fn failed_entries(&self) -> &[ImportFailedEntry] {
        &self.failed_entries
    }

    pub fn duplicate_candidate_count(&self) -> usize {
        self.duplicate_candidate_entries.len()
    }

    pub fn duplicate_candidate_entries(&self) -> &[ImportDuplicateCandidateEntry] {
        &self.duplicate_candidate_entries
    }

    pub fn was_cancelled(&self) -> bool {
        !self.cancelled_files.is_empty()
    }

    pub fn cancelled_count(&self) -> usize {
        self.cancelled_files.len()
    }

    pub fn cancelled_files(&self) -> &[PathBuf] {
        &self.cancelled_files
    }

    pub fn exact_duplicate_count(&self) -> usize {
        self.skipped_entries
            .iter()
            .filter(|entry| matches!(entry.reason, ImportSkipReason::ExactFileDuplicate { .. }))
            .count()
    }

    pub fn maintenance_errors(&self) -> &[String] {
        &self.maintenance_errors
    }

    pub fn vault_problems(&self) -> &[ImportVaultProblem] {
        &self.vault_problems
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateItemRecord {
    pub id: String,
    pub title: Option<String>,
    pub creator: Option<String>,
    pub year: Option<String>,
    pub saving_reason: Option<String>,
    pub review_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDefinition {
    pub name: String,
    pub aliases: Vec<String>,
    pub meaning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDefinition {
    pub name: String,
    pub purpose: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    id: String,
    name: String,
}

impl Collection {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLinkDefinition {
    pub link_type: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLink {
    link_type: String,
    target: String,
    label: String,
}

impl ItemLink {
    pub fn link_type(&self) -> &str {
        &self.link_type
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBudgetMode {
    Off,
    Cheap,
    Standard,
    Deep,
}

impl AiBudgetMode {
    fn as_log_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Cheap => "cheap",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

pub trait AiProvider {
    fn enrich(&self, request: AiProviderRequest) -> AiProviderResponse;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProviderRequest {
    pub item_id: String,
    pub budget_mode: AiBudgetMode,
    pub cleaned_text: Option<String>,
    pub raw_html: Option<String>,
    pub image_bytes: Option<Vec<u8>>,
    pub related_item_records: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiProviderResponse {
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub suggestions: Vec<AiMetadataSuggestion>,
    pub better_file_candidates: Vec<BetterFileCandidate>,
    pub estimated_cost_cents: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiMetadataSuggestion {
    pub field: String,
    pub suggested_value: String,
    pub confidence: f32,
    pub provenance: String,
}

impl AiMetadataSuggestion {
    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn suggested_value(&self) -> &str {
        &self.suggested_value
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    pub fn provenance(&self) -> &str {
        &self.provenance
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetterFileCandidate {
    pub source_link: String,
    pub reason: String,
    pub provenance: String,
}

impl BetterFileCandidate {
    pub fn source_link(&self) -> &str {
        &self.source_link
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn provenance(&self) -> &str {
        &self.provenance
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiEnrichmentResult {
    accepted_summary: Option<String>,
    accepted_tags: Vec<String>,
    accepted_metadata: Vec<AiMetadataSuggestion>,
    staged_suggestions: Vec<AiMetadataSuggestion>,
    better_file_candidates: Vec<BetterFileCandidate>,
    estimated_cost_cents: u32,
}

impl AiEnrichmentResult {
    pub fn accepted_summary(&self) -> Option<&str> {
        self.accepted_summary.as_deref()
    }

    pub fn accepted_tags(&self) -> Vec<&str> {
        self.accepted_tags.iter().map(String::as_str).collect()
    }

    pub fn accepted_metadata(&self) -> &[AiMetadataSuggestion] {
        &self.accepted_metadata
    }

    pub fn staged_suggestions(&self) -> &[AiMetadataSuggestion] {
        &self.staged_suggestions
    }

    pub fn better_file_candidates(&self) -> &[BetterFileCandidate] {
        &self.better_file_candidates
    }

    pub fn estimated_cost_cents(&self) -> u32 {
        self.estimated_cost_cents
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateCandidate {
    item_id: String,
    signal: String,
}

impl DuplicateCandidate {
    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn signal(&self) -> &str {
        &self.signal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualFallbackCapture {
    pub source_link: String,
    pub title: String,
    pub saving_reason: Option<String>,
    pub copied_text: Option<String>,
    pub copied_image: Option<CopiedImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedTextCapture {
    pub source_link: String,
    pub title: String,
    pub saving_reason: Option<String>,
    pub cleaned_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLinkCapture {
    pub source_link: String,
    pub title: String,
    pub saving_reason: Option<String>,
}

pub trait SourceExtractor {
    fn extract(&self, request: SourceExtractionRequest) -> SourceExtraction;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceExtractionRequest {
    pub source_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceExtraction {
    ExtractedText {
        title: Option<String>,
        cleaned_text: String,
    },
    NeedsManualFallback {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCaptureResult {
    Captured(SavedItem),
    NeedsManualFallback(ManualFallbackPrompt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualFallbackPrompt {
    source_link: String,
    title: String,
    saving_reason: Option<String>,
    reason: String,
}

impl ManualFallbackPrompt {
    pub fn source_link(&self) -> &str {
        &self.source_link
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn saving_reason(&self) -> Option<&str> {
        self.saving_reason.as_deref()
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdeaSourceCapture {
    source_link: String,
    title: String,
    saving_reason: Option<String>,
    copied_text: Option<String>,
    copied_image: Option<CopiedImage>,
    capture_method: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedImage {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedItem {
    id: String,
    home_subvault: String,
    item_folder: PathBuf,
}

impl SavedItem {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn home_subvault(&self) -> &str {
        &self.home_subvault
    }

    pub fn item_folder(&self) -> &Path {
        &self.item_folder
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuiltMetadataIndex {
    indexed_items: usize,
}

impl RebuiltMetadataIndex {
    pub fn indexed_items(&self) -> usize {
        self.indexed_items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    saved_item: SavedItem,
}

impl SearchResult {
    pub fn saved_item(&self) -> &SavedItem {
        &self.saved_item
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemRecordEntry {
    record_path: PathBuf,
    item_folder: PathBuf,
}

#[derive(Debug)]
pub enum VaultError {
    Io(io::Error),
    MissingRoot(PathBuf),
    NonEmptyRoot(PathBuf),
    MissingConfig(PathBuf),
    MissingSubvaults(PathBuf),
    MissingCollections(PathBuf),
    StructuralConflict(PathBuf),
    RepairProposalChanged(PathBuf),
    MalformedConfig { path: PathBuf, reason: String },
    UnsupportedFormat(PathBuf),
    MissingSourceFile(PathBuf),
    MissingFileName(PathBuf),
    UnsupportedImageFile(PathBuf),
    MissingImportFolder(PathBuf),
    SavedItemNotFound(String),
    CollectionNotFound(String),
    MalformedItemRecord(PathBuf),
    ItemRecordCodec(String),
    PreviewGeneration(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingRoot(path) => write!(f, "vault root does not exist: {}", path.display()),
            Self::NonEmptyRoot(path) => {
                write!(f, "vault folder is not empty: {}", path.display())
            }
            Self::MissingConfig(path) => {
                write!(f, "vault config is missing: {}", path.display())
            }
            Self::MissingSubvaults(path) => {
                write!(f, "subvaults directory is missing: {}", path.display())
            }
            Self::MissingCollections(path) => {
                write!(f, "collections directory is missing: {}", path.display())
            }
            Self::StructuralConflict(path) => write!(
                f,
                "required vault directory conflicts with an existing file: {}",
                path.display()
            ),
            Self::RepairProposalChanged(path) => write!(
                f,
                "vault structure changed after repair was proposed: {}",
                path.display()
            ),
            Self::MalformedConfig { path, reason } => {
                write!(f, "vault config is malformed: {}: {reason}", path.display())
            }
            Self::UnsupportedFormat(path) => {
                write!(
                    f,
                    "vault config has an unsupported format: {}",
                    path.display()
                )
            }
            Self::MissingSourceFile(path) => {
                write!(f, "source file does not exist: {}", path.display())
            }
            Self::MissingFileName(path) => {
                write!(f, "source file has no file name: {}", path.display())
            }
            Self::UnsupportedImageFile(path) => {
                write!(f, "unsupported image file: {}", path.display())
            }
            Self::MissingImportFolder(path) => {
                write!(f, "import folder does not exist: {}", path.display())
            }
            Self::SavedItemNotFound(id) => write!(f, "saved item not found: {id}"),
            Self::CollectionNotFound(id) => write!(f, "collection not found: {id}"),
            Self::MalformedItemRecord(path) => {
                write!(f, "item record is malformed: {}", path.display())
            }
            Self::ItemRecordCodec(reason) => write!(f, "item record codec failed: {reason}"),
            Self::PreviewGeneration(reason) => {
                write!(f, "thumbnail preview generation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for VaultError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for VaultError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn validate_vault_root(root: &Path) -> Result<(), VaultError> {
    validate_vault_config(root)?;

    let subvaults_path = root.join(SUBVAULTS_DIR);
    if !subvaults_path.is_dir() {
        return Err(VaultError::MissingSubvaults(subvaults_path));
    }

    let collections_path = root.join(COLLECTIONS_DIR);
    if !collections_path.is_dir() {
        return Err(VaultError::MissingCollections(collections_path));
    }

    Ok(())
}

fn validate_vault_config(root: &Path) -> Result<(), VaultError> {
    if !root.exists() {
        return Err(VaultError::MissingRoot(root.to_path_buf()));
    }

    let config_path = root.join(VAULT_CONFIG_FILE);
    if !config_path.is_file() {
        return Err(VaultError::MissingConfig(config_path));
    }

    let config_text = fs::read_to_string(&config_path)?;
    let config = toml::from_str::<toml::Value>(&config_text).map_err(|error| {
        VaultError::MalformedConfig {
            path: config_path.clone(),
            reason: error.to_string(),
        }
    })?;
    if config
        .get("format_version")
        .and_then(toml::Value::as_integer)
        != Some(2)
    {
        return Err(VaultError::UnsupportedFormat(config_path));
    }

    Ok(())
}

fn missing_repairable_directories(root: &Path) -> Result<Vec<PathBuf>, VaultError> {
    let mut missing = Vec::new();
    for directory in [SUBVAULTS_DIR, COLLECTIONS_DIR] {
        let path = root.join(directory);
        if path.is_dir() {
            continue;
        }
        if path.exists() {
            return Err(VaultError::StructuralConflict(path));
        }
        missing.push(path);
    }
    Ok(missing)
}

fn default_config() -> String {
    format!("format_version = 2\nname = \"{DEFAULT_VAULT_NAME}\"\n")
}

fn artwork_folder_name(creator: Option<&str>, year: Option<&str>, title: &str) -> String {
    format!(
        "{} - {} - {}",
        readable_part(creator, "Unknown Creator"),
        readable_part(year, "Unknown Year"),
        readable_part(Some(title), "Untitled")
    )
}

fn folder_rename_suggestion(
    item_folder: &Path,
    item_type: &str,
    creator: &str,
    year: &str,
    title: &str,
) -> Option<String> {
    if item_type != "artwork" {
        return None;
    }

    let suggested = artwork_folder_name(Some(creator), Some(year), title);
    let current = item_folder.file_name()?.to_string_lossy();
    if suggested == current {
        None
    } else {
        Some(suggested)
    }
}

fn readable_part(value: Option<&str>, fallback: &str) -> String {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.replace(['/', '\\'], "-")
    }
}

fn unique_folder_path(root: &Path, preferred_name: &str) -> PathBuf {
    let preferred = root.join(preferred_name);
    if !preferred.exists() {
        return preferred;
    }

    for suffix in 2.. {
        let candidate = root.join(format!("{preferred_name} ({suffix})"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded suffix search should always find a free path")
}

fn new_item_id() -> String {
    format!("item-{}", uuid::Uuid::new_v4())
}

fn inferred_artwork_item(source_file: PathBuf, metadata: &ArtworkImportMetadata) -> AddArtworkItem {
    let inferred = infer_artwork_metadata(&source_file);
    AddArtworkItem {
        source_file,
        home_subvault: "Paintings".to_string(),
        creator: metadata.creator.clone().or(inferred.creator),
        year: metadata.year.clone().or(inferred.year),
        title: inferred.title,
        saving_reason: metadata.saving_reason.clone(),
    }
}

fn discover_import_entries(
    source_folder: &Path,
    source_files: &mut Vec<PathBuf>,
    skipped_entries: &mut Vec<ImportSkippedEntry>,
    failed_entries: &mut Vec<ImportFailedEntry>,
) {
    let entries = match fs::read_dir(source_folder) {
        Ok(entries) => entries,
        Err(error) => {
            failed_entries.push(ImportFailedEntry {
                path: source_folder.to_path_buf(),
                error: error.to_string(),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failed_entries.push(ImportFailedEntry {
                    path: source_folder.to_path_buf(),
                    error: error.to_string(),
                });
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                failed_entries.push(ImportFailedEntry {
                    path: entry.path(),
                    error: error.to_string(),
                });
                continue;
            }
        };
        if file_type.is_symlink() {
            skipped_entries.push(ImportSkippedEntry {
                path: entry.path(),
                reason: ImportSkipReason::SymbolicLink,
            });
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            discover_import_entries(&path, source_files, skipped_entries, failed_entries);
        } else if file_type.is_file() && is_supported_image_file(&path) {
            source_files.push(path);
        } else if file_type.is_file() {
            skipped_entries.push(ImportSkippedEntry {
                path,
                reason: ImportSkipReason::UnsupportedFile,
            });
        }
    }
}

fn imported_at() -> String {
    let nanoseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    nanoseconds.to_string()
}

fn file_fingerprint(path: &Path) -> Result<String, VaultError> {
    let bytes = fs::read(path)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

#[derive(Serialize)]
struct ArtworkFrontmatter<'a> {
    id: &'a str,
    item_type: &'static str,
    home_subvault: &'a str,
    title: &'a str,
    creator: &'a str,
    year: &'a str,
    primary_file: &'a str,
    import_original_filename: &'a str,
    import_source_path: String,
    import_source_folder: String,
    imported_at: &'a str,
    file_fingerprint: &'a str,
    duplicate_candidates: Vec<String>,
    review_reasons: Vec<String>,
    review_status: &'static str,
}

fn artwork_record(
    id: &str,
    item: &AddArtworkItem,
    primary_file: &str,
    import_original_filename: &str,
    import_source_path: &Path,
    imported_at: &str,
    file_fingerprint: &str,
    duplicate_candidates: &[DuplicateCandidate],
) -> Result<String, VaultError> {
    let creator = item.creator.as_deref().unwrap_or("Unknown Creator");
    let year = item.year.as_deref().unwrap_or("Unknown Year");
    let saving_reason = item.saving_reason.as_deref().unwrap_or("");
    let review_status = if duplicate_candidates.is_empty()
        && creator != "Unknown Creator"
        && year != "Unknown Year"
    {
        "reviewed"
    } else {
        "needs-review"
    };
    let import_source_folder = import_source_path
        .parent()
        .map(Path::display)
        .map(|display| display.to_string())
        .unwrap_or_default();

    let frontmatter = ArtworkFrontmatter {
        id,
        item_type: "artwork",
        home_subvault: &item.home_subvault,
        title: &item.title,
        creator,
        year,
        primary_file,
        import_original_filename,
        import_source_path: import_source_path.display().to_string(),
        import_source_folder,
        imported_at,
        file_fingerprint,
        duplicate_candidates: duplicate_candidates
            .iter()
            .map(|candidate| format!("{} | {}", candidate.item_id, candidate.signal))
            .collect(),
        review_reasons: duplicate_candidates
            .iter()
            .map(|candidate| {
                format!(
                    "duplicate-candidate | {} | {}",
                    candidate.item_id, candidate.signal
                )
            })
            .collect(),
        review_status,
    };
    let yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|error| VaultError::ItemRecordCodec(error.to_string()))?;

    Ok(format!(
        "---\n{yaml}---\n\n# {title}\n\n## Saving Reason\n\n{saving_reason}\n",
        title = item.title,
    ))
}

fn idea_source_record(
    id: &str,
    capture: &IdeaSourceCapture,
    source_copy: Option<&str>,
    primary_file: Option<&str>,
    duplicate_candidates: &[DuplicateCandidate],
) -> String {
    let saving_reason = capture.saving_reason.as_deref().unwrap_or("");
    let source_copy = source_copy.unwrap_or("");
    let primary_file = primary_file.unwrap_or("");
    let duplicate_candidates = duplicate_candidate_frontmatter(duplicate_candidates);

    format!(
        "---\n\
id: {id}\n\
item_type: idea\n\
home_subvault: Idea Sources\n\
title: {title}\n\
creator: Unknown Creator\n\
year: \"Unknown Year\"\n\
source_link: {source_link}\n\
source_copy: {source_copy}\n\
primary_file: {primary_file}\n\
capture_method: {capture_method}\n\
duplicate_candidates: {duplicate_candidates}\n\
review_status: needs-review\n\
---\n\
\n\
# {title}\n\
\n\
## Saving Reason\n\
\n\
{saving_reason}\n",
        title = capture.title,
        source_link = capture.source_link,
        capture_method = capture.capture_method,
    )
}

fn collection_record(id: &str, collection: &CollectionDefinition, item_ids: &[String]) -> String {
    let description = collection.description.as_deref().unwrap_or("");
    let mut record = format!(
        "---\n\
id: {id}\n\
name: {name}\n\
purpose: {purpose}\n\
---\n\
\n\
# {name}\n\
\n\
## Description\n\
\n\
{description}\n\
\n\
## Items\n\
\n",
        name = collection.name,
        purpose = collection.purpose,
    );

    for item_id in item_ids {
        record.push_str(&format!("- {item_id}\n"));
    }

    record
}

fn collection_items(record: &str) -> Vec<String> {
    markdown_list_section(record, "Items")
}

fn replace_collection_items(record: &str, item_ids: &[String]) -> String {
    replace_markdown_list_section(record, "Items", item_ids)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InferredArtworkMetadata {
    creator: Option<String>,
    year: Option<String>,
    title: String,
}

fn infer_artwork_metadata(path: &Path) -> InferredArtworkMetadata {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".to_string());
    let parts: Vec<_> = stem.split(" - ").collect();

    if parts.len() >= 3 {
        return InferredArtworkMetadata {
            creator: Some(parts[0].trim().to_string()),
            year: Some(parts[1].trim().to_string()),
            title: parts[2..].join(" - ").trim().to_string(),
        };
    }

    InferredArtworkMetadata {
        creator: None,
        year: None,
        title: stem,
    }
}

fn is_supported_image_file(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    matches!(
        extension.to_string_lossy().to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif"
    )
}

fn normalize_search_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn escape_index_field(field: &str) -> String {
    field
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_index_field(field: &str) -> String {
    let mut output = String::new();
    let mut chars = field.chars();
    while let Some(char) = chars.next() {
        if char != '\\' {
            output.push(char);
            continue;
        }

        match chars.next() {
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn saved_item_from_record(record: &ItemRecordEntry, text: &str) -> Result<SavedItem, VaultError> {
    Ok(SavedItem {
        id: required_frontmatter_value(&record.record_path, text, "id")?,
        home_subvault: required_frontmatter_value(&record.record_path, text, "home_subvault")?,
        item_folder: record.item_folder.clone(),
    })
}

fn required_frontmatter_value(
    record_path: &Path,
    record: &str,
    key: &str,
) -> Result<String, VaultError> {
    frontmatter_value(record, key)
        .ok_or_else(|| VaultError::MalformedItemRecord(record_path.to_path_buf()))
}

fn frontmatter_list(record: &str, key: &str) -> Vec<String> {
    let Some(value) = frontmatter_mapping(record).and_then(|mapping| {
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .cloned()
    }) else {
        return Vec::new();
    };

    match value {
        serde_yaml::Value::Sequence(values) => {
            values.into_iter().filter_map(yaml_scalar_string).collect()
        }
        value => yaml_scalar_string(value)
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn frontmatter_value(record: &str, key: &str) -> Option<String> {
    let mapping = frontmatter_mapping(record)?;
    mapping
        .get(serde_yaml::Value::String(key.to_string()))
        .cloned()
        .and_then(yaml_scalar_string)
}

fn frontmatter_mapping(record: &str) -> Option<serde_yaml::Mapping> {
    let frontmatter = record.strip_prefix("---\n")?.split_once("\n---\n")?.0;
    serde_yaml::from_str(frontmatter).ok()
}

fn update_frontmatter_values(
    record: &str,
    values: &[(&str, serde_yaml::Value)],
) -> Result<String, VaultError> {
    let without_opening = record
        .strip_prefix("---\n")
        .ok_or_else(|| VaultError::ItemRecordCodec("missing frontmatter opening".to_string()))?;
    let (frontmatter, body) = without_opening
        .split_once("\n---\n")
        .ok_or_else(|| VaultError::ItemRecordCodec("missing frontmatter closing".to_string()))?;
    serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter)
        .map_err(|error| VaultError::ItemRecordCodec(error.to_string()))?;
    let mut lines = frontmatter
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    for (key, value) in values {
        let mut replacement = serde_yaml::Mapping::new();
        replacement.insert(serde_yaml::Value::String((*key).to_string()), value.clone());
        let replacement = serde_yaml::to_string(&replacement)
            .map_err(|error| VaultError::ItemRecordCodec(error.to_string()))?
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if let Some(start) = lines
            .iter()
            .position(|line| top_level_yaml_key(line).as_deref() == Some(*key))
        {
            let end = lines[start + 1..]
                .iter()
                .position(|line| top_level_yaml_key(line).is_some())
                .map(|offset| start + 1 + offset)
                .unwrap_or(lines.len());
            let comments = lines[start..end]
                .iter()
                .filter(|line| line.trim_start().starts_with('#'))
                .cloned()
                .collect::<Vec<_>>();
            lines.splice(start..end, replacement.into_iter().chain(comments));
        } else {
            lines.extend(replacement);
        }
    }
    Ok(format!("---\n{}\n---\n{body}", lines.join("\n")))
}

fn top_level_yaml_key(line: &str) -> Option<String> {
    if line.is_empty()
        || line.chars().next().is_some_and(char::is_whitespace)
        || line.starts_with('-')
        || line.starts_with('#')
    {
        return None;
    }
    let mapping = serde_yaml::from_str::<serde_yaml::Mapping>(line).ok()?;
    if mapping.len() != 1 {
        return None;
    }
    mapping
        .into_iter()
        .next()?
        .0
        .as_str()
        .map(ToOwned::to_owned)
}

fn write_thumbnail_placeholder(path: &Path) -> Result<(), VaultError> {
    let mut placeholder = image::RgbaImage::from_pixel(480, 320, image::Rgba([238, 240, 239, 255]));
    for offset in 0..320_u32 {
        let first_x = offset + 80;
        let second_x = 399 - offset;
        for thickness in 0..3_u32 {
            if first_x + thickness < 480 {
                placeholder.put_pixel(
                    first_x + thickness,
                    offset,
                    image::Rgba([137, 147, 142, 255]),
                );
            }
            if second_x + thickness < 480 {
                placeholder.put_pixel(
                    second_x + thickness,
                    offset,
                    image::Rgba([137, 147, 142, 255]),
                );
            }
        }
    }
    placeholder
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|error| VaultError::PreviewGeneration(error.to_string()))
}

fn yaml_scalar_string(value: serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Null => Some(String::new()),
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        serde_yaml::Value::String(value) => Some(value),
        _ => None,
    }
}

fn markdown_section(record: &str, heading: &str) -> Option<String> {
    let target = format!("## {heading}");
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in record.lines() {
        if line == target {
            in_section = true;
            continue;
        }

        if in_section && line.starts_with("## ") {
            break;
        }

        if in_section {
            lines.push(line);
        }
    }

    let section = lines.join("\n").trim().to_string();
    if section.is_empty() {
        None
    } else {
        Some(section)
    }
}

fn replace_markdown_section(record: &str, heading: &str, value: &str) -> String {
    let target = format!("## {heading}");
    let mut output = Vec::new();
    let mut lines = record.lines().peekable();

    while let Some(line) = lines.next() {
        output.push(line.to_string());
        if line != target {
            continue;
        }

        while matches!(lines.peek(), Some(next) if next.is_empty()) {
            lines.next();
        }

        output.push(String::new());
        output.push(value.to_string());

        while matches!(lines.peek(), Some(next) if !next.starts_with("## ")) {
            lines.next();
        }
    }

    output.join("\n") + "\n"
}

fn replace_or_append_markdown_section(record: &str, heading: &str, value: &str) -> String {
    let target = format!("## {heading}");
    if record.lines().any(|line| line == target) {
        return replace_markdown_section(record, heading, value);
    }

    let mut output = record.trim_end().to_string();
    output.push_str("\n\n");
    output.push_str(&target);
    output.push_str("\n\n");
    output.push_str(value);
    output.push('\n');
    output
}

fn markdown_list_section(record: &str, heading: &str) -> Vec<String> {
    let target = format!("## {heading}");
    let mut in_section = false;
    let mut items = Vec::new();

    for line in record.lines() {
        if line == target {
            in_section = true;
            continue;
        }

        if in_section && line.starts_with("## ") {
            break;
        }

        if in_section {
            if let Some(item) = line.strip_prefix("- ") {
                items.push(item.to_string());
            }
        }
    }

    items
}

fn replace_markdown_list_section(record: &str, heading: &str, items: &[String]) -> String {
    let target = format!("## {heading}");
    let mut output = Vec::new();
    let mut lines = record.lines().peekable();

    while let Some(line) = lines.next() {
        output.push(line.to_string());
        if line != target {
            continue;
        }

        while matches!(lines.peek(), Some(next) if next.is_empty()) {
            lines.next();
        }

        output.push(String::new());
        for item in items {
            output.push(format!("- {item}"));
        }

        while matches!(lines.peek(), Some(next) if !next.starts_with("## ")) {
            lines.next();
        }
    }

    output.join("\n") + "\n"
}

fn replace_or_append_markdown_list_section(
    record: &str,
    heading: &str,
    items: &[String],
) -> String {
    let target = format!("## {heading}");
    if record.lines().any(|line| line == target) {
        return replace_markdown_list_section(record, heading, items);
    }

    let mut output = record.trim_end().to_string();
    output.push_str("\n\n");
    output.push_str(&target);
    output.push_str("\n\n");
    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
    output
}

fn item_link_line(link: &ItemLinkDefinition) -> String {
    format!("{} | {} | {}", link.link_type, link.label, link.target)
}

fn item_links(record: &str) -> Vec<ItemLink> {
    markdown_list_section(record, "Item Links")
        .into_iter()
        .filter_map(|line| {
            let mut parts = line.splitn(3, " | ").map(str::trim);
            Some(ItemLink {
                link_type: parts.next()?.to_string(),
                label: parts.next()?.to_string(),
                target: parts.next()?.to_string(),
            })
        })
        .collect()
}

fn metadata_suggestion_line(suggestion: &AiMetadataSuggestion) -> String {
    format!(
        "{} | {} | {:.2} | {}",
        suggestion.field, suggestion.suggested_value, suggestion.confidence, suggestion.provenance
    )
}

fn metadata_suggestions(record: &str) -> Vec<AiMetadataSuggestion> {
    markdown_list_section(record, "Metadata Suggestions")
        .into_iter()
        .filter_map(|line| {
            let mut parts = line.splitn(4, " | ").map(str::trim);
            Some(AiMetadataSuggestion {
                field: parts.next()?.to_string(),
                suggested_value: parts.next()?.to_string(),
                confidence: parts.next()?.parse().ok()?,
                provenance: parts.next()?.to_string(),
            })
        })
        .collect()
}

fn metadata_provenance(record: &str) -> Vec<AiMetadataSuggestion> {
    markdown_list_section(record, "Metadata Provenance")
        .into_iter()
        .filter_map(|line| {
            let mut parts = line.splitn(4, " | ").map(str::trim);
            Some(AiMetadataSuggestion {
                field: parts.next()?.to_string(),
                suggested_value: parts.next()?.to_string(),
                confidence: parts.next()?.parse().ok()?,
                provenance: parts.next()?.to_string(),
            })
        })
        .collect()
}

fn split_metadata_suggestions(
    record: &str,
    suggestions: Vec<AiMetadataSuggestion>,
) -> (Vec<AiMetadataSuggestion>, Vec<AiMetadataSuggestion>) {
    let mut accepted = Vec::new();
    let mut staged = Vec::new();

    for suggestion in suggestions {
        if can_accept_metadata_suggestion(record, &suggestion) {
            accepted.push(suggestion);
        } else {
            staged.push(suggestion);
        }
    }

    (accepted, staged)
}

fn can_accept_metadata_suggestion(record: &str, suggestion: &AiMetadataSuggestion) -> bool {
    if suggestion.confidence < 0.90 {
        return false;
    }

    if !matches!(suggestion.field.as_str(), "creator" | "year" | "title") {
        return false;
    }

    frontmatter_value(record, &suggestion.field)
        .map(|value| is_unknown_metadata_value(&value))
        .unwrap_or(false)
}

fn is_unknown_metadata_value(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || value.starts_with("Unknown") || value.starts_with("Untitled")
}

fn better_file_candidate_line(candidate: &BetterFileCandidate) -> String {
    format!(
        "{} | {} | {}",
        candidate.source_link, candidate.reason, candidate.provenance
    )
}

fn better_file_candidates(record: &str) -> Vec<BetterFileCandidate> {
    markdown_list_section(record, "Better File Candidates")
        .into_iter()
        .filter_map(|line| {
            let mut parts = line.splitn(3, " | ").map(str::trim);
            Some(BetterFileCandidate {
                source_link: parts.next()?.to_string(),
                reason: parts.next()?.to_string(),
                provenance: parts.next()?.to_string(),
            })
        })
        .collect()
}

fn duplicate_candidate_frontmatter(candidates: &[DuplicateCandidate]) -> String {
    candidates
        .iter()
        .map(|candidate| format!("{} | {}", candidate.item_id, candidate.signal))
        .collect::<Vec<_>>()
        .join(", ")
}

fn duplicate_candidates(record: &str) -> Vec<DuplicateCandidate> {
    frontmatter_list(record, "duplicate_candidates")
        .into_iter()
        .filter_map(|entry| {
            let (item_id, signal) = entry.split_once(" | ")?;
            Some(DuplicateCandidate {
                item_id: item_id.to_string(),
                signal: signal.to_string(),
            })
        })
        .collect()
}

fn activity_log_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn push_duplicate_candidate(
    candidates: &mut Vec<DuplicateCandidate>,
    item_id: String,
    signal: &str,
) {
    if candidates
        .iter()
        .any(|candidate| candidate.item_id == item_id && candidate.signal == signal)
    {
        return;
    }

    candidates.push(DuplicateCandidate {
        item_id,
        signal: signal.to_string(),
    });
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for char in value.chars() {
        if char.is_ascii_alphanumeric() {
            slug.push(char.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn canonical_tag(registry: &[TagDefinition], input: &str) -> String {
    let input = input.trim();
    let normalized_input = input.to_ascii_lowercase();
    registry
        .iter()
        .find(|tag| {
            tag.name.to_ascii_lowercase() == normalized_input
                || tag
                    .aliases
                    .iter()
                    .any(|alias| alias.to_ascii_lowercase() == normalized_input)
        })
        .map(|tag| tag.name.clone())
        .unwrap_or_else(|| input.to_string())
}

fn parse_tag_registry(registry: &str) -> Result<Vec<TagDefinition>, VaultError> {
    let mut tags = Vec::new();
    let mut current_name: Option<String> = None;
    let mut aliases = Vec::new();
    let mut meaning_lines = Vec::new();

    for line in registry.lines() {
        if let Some(name) = line.strip_prefix("## ") {
            push_parsed_tag(
                &mut tags,
                current_name.take(),
                std::mem::take(&mut aliases),
                std::mem::take(&mut meaning_lines),
            );
            current_name = Some(name.to_string());
            continue;
        }

        if let Some(alias_text) = line.strip_prefix("Aliases: ") {
            aliases = alias_text
                .split(',')
                .map(str::trim)
                .filter(|alias| !alias.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            continue;
        }

        if current_name.is_some() && !line.is_empty() && line != "# Tag Registry" {
            meaning_lines.push(line.to_string());
        }
    }

    push_parsed_tag(&mut tags, current_name, aliases, meaning_lines);
    Ok(tags)
}

fn push_parsed_tag(
    tags: &mut Vec<TagDefinition>,
    name: Option<String>,
    aliases: Vec<String>,
    meaning_lines: Vec<String>,
) {
    let Some(name) = name else {
        return;
    };

    let meaning = if meaning_lines.is_empty() {
        None
    } else {
        Some(meaning_lines.join("\n"))
    };

    tags.push(TagDefinition {
        name,
        aliases,
        meaning,
    });
}
