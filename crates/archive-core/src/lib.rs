use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

const VAULT_CONFIG_FILE: &str = "vault.toml";
const SUBVAULTS_DIR: &str = "subvaults";
const COLLECTIONS_DIR: &str = "collections";
const TRASH_DIR: &str = "trash";
const HIDDEN_STATE_DIR: &str = ".gruenesgewolbe";
const METADATA_INDEX_FILE: &str = "metadata-index.tsv";
const AI_COST_LOG_FILE: &str = "ai-cost-log.tsv";
const ACTIVITY_LOG_FILE: &str = "activity-log.tsv";
const THUMBNAILS_DIR: &str = "thumbnails";
const MAX_THUMBNAIL_SOURCE_PIXELS: u64 = 24_000_000;
const TAG_REGISTRY_FILE: &str = "tag-registry.md";
const DEFAULT_VAULT_NAME: &str = "Personal Archive";
const MAX_IDEA_SOURCE_TEXT_BYTES: u64 = 2 * 1024 * 1024;

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

    pub fn activity_log_path(&self) -> PathBuf {
        self.root.join(HIDDEN_STATE_DIR).join(ACTIVITY_LOG_FILE)
    }

    pub fn vault_problems(&self) -> Result<Vec<VaultProblem>, VaultError> {
        let mut problems = self.item_record_scan()?.vault_problems;
        problems.extend(self.trashed_item_record_scan()?.vault_problems);
        problems.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(problems)
    }

    pub fn move_item_to_trash(&self, id: &str) -> Result<SavedItem, VaultError> {
        let record = self.find_active_record(id)?;
        let home_subvault =
            required_frontmatter_value(&record.entry.record_path, &record.text, "home_subvault")?;
        let folder_name = record
            .entry
            .item_folder
            .file_name()
            .ok_or_else(|| VaultError::SavedItemNotFound(id.to_string()))?;
        let destination_root = self.root.join(TRASH_DIR).join(&home_subvault);
        fs::create_dir_all(&destination_root)?;
        let destination = unique_folder_path(&destination_root, &folder_name.to_string_lossy());
        fs::rename(&record.entry.item_folder, &destination)?;
        self.rebuild_metadata_index()?;
        self.append_activity_log(&format!("trash-item\t{}\t{}", id, destination.display()))?;
        Ok(SavedItem {
            id: id.to_string(),
            home_subvault,
            item_folder: destination,
        })
    }

    pub fn restore_trashed_item(&self, id: &str) -> Result<SavedItem, VaultError> {
        let record = self.find_trashed_record(id)?;
        let home_subvault =
            required_frontmatter_value(&record.entry.record_path, &record.text, "home_subvault")?;
        let folder_name = record
            .entry
            .item_folder
            .file_name()
            .ok_or_else(|| VaultError::TrashedItemNotFound(id.to_string()))?;
        let destination_root = self
            .root
            .join(SUBVAULTS_DIR)
            .join(&home_subvault)
            .join("items");
        fs::create_dir_all(&destination_root)?;
        let destination = unique_folder_path(&destination_root, &folder_name.to_string_lossy());
        fs::rename(&record.entry.item_folder, &destination)?;
        self.rebuild_metadata_index()?;
        self.append_activity_log(&format!("restore-item\t{}\t{}", id, destination.display()))?;
        Ok(SavedItem {
            id: id.to_string(),
            home_subvault,
            item_folder: destination,
        })
    }

    pub fn permanently_delete_trashed_item(
        &self,
        id: &str,
        confirmed_id: &str,
    ) -> Result<PermanentDeletion, VaultError> {
        if id != confirmed_id {
            return Err(VaultError::PermanentDeletionNotConfirmed);
        }
        let target = self.find_trashed_record(id)?;
        let active_scan = self.item_record_scan()?;
        let trash_scan = self.trashed_item_record_scan()?;
        if let Some(problem) = active_scan.vault_problems.first().or_else(|| {
            trash_scan
                .vault_problems
                .iter()
                .find(|problem| problem.path != target.entry.record_path)
        }) {
            return Err(VaultError::ReferenceCleanupBlocked(problem.path.clone()));
        }

        let impact = PermanentDeletion {
            id: id.to_string(),
            collections: self.collection_names_for_item(id, &target.text)?,
            incoming_item_links: self.incoming_item_links(id)?,
        };
        let mut mutations: Vec<(PathBuf, String, String)> = Vec::new();
        for entry in fs::read_dir(self.root.join(COLLECTIONS_DIR))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let original = fs::read_to_string(&path)?;
            let mut items = collection_items(&original);
            let previous_len = items.len();
            items.retain(|item_id| item_id != id);
            if items.len() != previous_len {
                mutations.push((
                    path,
                    original.clone(),
                    replace_collection_items(&original, &items),
                ));
            }
        }
        for record in active_scan
            .valid_records
            .into_iter()
            .chain(trash_scan.valid_records)
        {
            if record.entry.record_path == target.entry.record_path {
                continue;
            }
            let lines = markdown_list_section(&record.text, "Item Links");
            let retained: Vec<String> = lines
                .iter()
                .filter(|line| parse_item_link_line(line).map_or(true, |link| link.target() != id))
                .cloned()
                .collect();
            if retained.len() != lines.len() {
                let updated =
                    replace_or_append_markdown_list_section(&record.text, "Item Links", &retained);
                mutations.push((record.entry.record_path, record.text, updated));
            }
        }

        let mut applied: Vec<(PathBuf, String)> = Vec::new();
        for (path, original, updated) in &mutations {
            if let Err(error) = atomic_write(path, updated.as_bytes()) {
                rollback_text_mutations(&applied)?;
                return Err(error);
            }
            applied.push((path.clone(), original.clone()));
        }
        let index_path = self.root.join(HIDDEN_STATE_DIR).join(METADATA_INDEX_FILE);
        let log_path = self.activity_log_path();
        let index_before = fs::read(&index_path).ok();
        let log_before = fs::read(&log_path).ok();
        let maintenance = self
            .rebuild_metadata_index()
            .and_then(|_| self.append_activity_log(&format!("permanently-delete-item\t{}", id)));
        if let Err(error) = maintenance {
            rollback_text_mutations(&applied)?;
            restore_file_snapshot(&index_path, index_before.as_deref())?;
            restore_file_snapshot(&log_path, log_before.as_deref())?;
            return Err(error);
        }
        if let Err(error) = fs::remove_dir_all(&target.entry.item_folder) {
            rollback_text_mutations(&applied)?;
            restore_file_snapshot(&index_path, index_before.as_deref())?;
            restore_file_snapshot(&log_path, log_before.as_deref())?;
            return Err(VaultError::Io(error));
        }
        Ok(impact)
    }

    pub fn list_trashed_items(&self) -> Result<Vec<TrashedItem>, VaultError> {
        let mut items = Vec::new();
        for record in self.trashed_item_record_scan()?.valid_records {
            let id = required_frontmatter_value(&record.entry.record_path, &record.text, "id")?;
            items.push(TrashedItem {
                saved_item: saved_item_from_record(&record.entry, &record.text)?,
                title: required_frontmatter_value(
                    &record.entry.record_path,
                    &record.text,
                    "title",
                )?,
                creator: required_frontmatter_value(
                    &record.entry.record_path,
                    &record.text,
                    "creator",
                )?,
                year: required_frontmatter_value(&record.entry.record_path, &record.text, "year")?,
                review_status: derived_review_status(&review_reasons(&record.text)).to_string(),
                tags: frontmatter_list(&record.text, "tags"),
                collections: self.collection_names_for_item(&id, &record.text)?,
                incoming_item_links: self.incoming_item_links(&id)?,
            });
        }
        items.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(items)
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
        self.preserve_artwork_item(item, true, &[], Vec::new(), None, None)
            .map(|outcome| outcome.saved_item)
    }

    fn preserve_artwork_item(
        &self,
        item: AddArtworkItem,
        refresh_metadata_index: bool,
        excluded_duplicate_candidate_ids: &[String],
        additional_review_reasons: Vec<ReviewReason>,
        source_link: Option<&str>,
        capture_method: Option<&str>,
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
            Some(&file_fingerprint),
            item.creator.as_deref(),
            item.year.as_deref(),
            Some(item.title.as_str()),
            source_link.is_none().then_some(item.source_file.as_path()),
            source_link,
            excluded_duplicate_candidate_ids,
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
            &additional_review_reasons,
            source_link,
            capture_method,
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

        self.add_artwork_files_with_options(
            source_files,
            ArtworkImportOptions {
                metadata,
                exact_duplicate_policy: ExactDuplicatePolicy::Skip,
            },
        )
        .map(|summary| summary.0.imported_items)
    }

    pub fn add_artwork_files_with_options(
        &self,
        source_files: impl IntoIterator<Item = PathBuf>,
        options: ArtworkImportOptions,
    ) -> Result<SelectedFileImportSummary, VaultError> {
        let mut supported_files = Vec::new();
        let mut skipped_entries = Vec::new();
        for source_file in source_files {
            if is_supported_image_file(&source_file) {
                supported_files.push(source_file);
            } else {
                skipped_entries.push(ImportSkippedEntry {
                    path: source_file,
                    reason: ImportSkipReason::UnsupportedFile,
                });
            }
        }

        self.process_paintings_import(
            "selected-files",
            ImportActivityKind::SelectedFiles,
            supported_files,
            skipped_entries,
            Vec::new(),
            options,
            |_| ImportRunAction::Continue,
        )
        .map(SelectedFileImportSummary)
    }

    pub fn import_paintings_folder(
        &self,
        source_folder: impl AsRef<Path>,
    ) -> Result<Vec<SavedItem>, VaultError> {
        let source_folder = source_folder.as_ref();
        let imported = self
            .run_paintings_import(source_folder, |_| ImportRunAction::Continue)?
            .0
            .imported_items;
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
            ArtworkImportOptions {
                metadata,
                exact_duplicate_policy: ExactDuplicatePolicy::Skip,
            },
            on_progress,
        )
    }

    pub fn run_paintings_import_with_options<F>(
        &self,
        source_folder: impl AsRef<Path>,
        options: ArtworkImportOptions,
        on_progress: F,
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
        discover_import_entries(
            source_folder,
            &mut source_files,
            &mut skipped_entries,
            &mut failed_entries,
        );
        source_files.sort();
        skipped_entries.sort_by(|left, right| left.path.cmp(&right.path));
        self.process_paintings_import(
            &source_folder.display().to_string(),
            ImportActivityKind::FolderRun,
            source_files,
            skipped_entries,
            failed_entries,
            options,
            on_progress,
        )
        .map(ImportRunSummary)
    }

    fn process_paintings_import<F>(
        &self,
        source_label: &str,
        activity_kind: ImportActivityKind,
        source_files: Vec<PathBuf>,
        mut skipped_entries: Vec<ImportSkippedEntry>,
        mut failed_entries: Vec<ImportFailedEntry>,
        options: ArtworkImportOptions,
        mut on_progress: F,
    ) -> Result<ArtworkImportOutcome, VaultError>
    where
        F: FnMut(&ImportProgress) -> ImportRunAction,
    {
        let mut maintenance_errors = Vec::new();
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
            if options.exact_duplicate_policy == ExactDuplicatePolicy::Skip {
                if let Some(existing_item_id) = duplicate_check.existing_item_ids.first() {
                    skipped_entries.push(ImportSkippedEntry {
                        path: source_file.clone(),
                        reason: ImportSkipReason::ExactFileDuplicate {
                            existing_item_id: existing_item_id.clone(),
                        },
                    });
                    continue;
                }
            }
            let (item, metadata_conflicts) =
                inferred_artwork_item(source_file.clone(), &options.metadata);
            match self.preserve_artwork_item(
                item,
                false,
                &duplicate_check.existing_item_ids,
                metadata_conflicts,
                None,
                None,
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
        let event_name = match (activity_kind, cancelled_files.is_empty()) {
            (ImportActivityKind::FolderRun, true) => "import-run-completed",
            (ImportActivityKind::FolderRun, false) => "import-run-cancelled",
            (ImportActivityKind::SelectedFiles, true) => "selected-files-import-completed",
            (ImportActivityKind::SelectedFiles, false) => "selected-files-import-cancelled",
        };
        let summary_event = format!(
            "{event_name}\t{}\timported={}\tskipped={}\tduplicate-candidates={}\tcancelled={}\tfailed={}",
            activity_log_field(source_label),
            imported_items.len(),
            skipped_entries.len(),
            duplicate_candidate_entries.len(),
            cancelled_files.len(),
            failed_entries.len()
        );
        if let Err(error) = self.append_activity_log(&summary_event) {
            maintenance_errors.push(format!("activity-log: {error}"));
        }

        Ok(ArtworkImportOutcome {
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

    /// Capture an image explicitly into Paintings, without inferring a destination.
    pub fn capture_artwork_fallback(
        &self,
        capture: ManualFallbackCapture,
    ) -> Result<SavedItem, VaultError> {
        if capture.copied_text.as_ref().is_some_and(|text| !text.trim().is_empty()) {
            return Err(VaultError::InvalidItemRecordEdit(vec![
                "Use Idea Sources to preserve pasted text; Paintings requires an image".to_string(),
            ]));
        }
        let image = capture.copied_image.filter(|image| !image.bytes.is_empty())
            .ok_or_else(|| VaultError::InvalidItemRecordEdit(vec![
                "Choose or paste an image to save in Paintings".to_string(),
            ]))?;
        self.capture_extracted_image(capture.source_link, capture.title,
            capture.saving_reason, image.file_name, image.bytes, "manual-fallback")
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
            SourceExtraction::ExtractedImage {
                title,
                file_name,
                bytes,
            } => self
                .capture_extracted_image(
                    capture.source_link,
                    title.unwrap_or(capture.title),
                    capture.saving_reason,
                    file_name,
                    bytes,
                    "extracted-image",
                )
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

    fn capture_extracted_image(
        &self,
        source_link: String,
        title: String,
        saving_reason: Option<String>,
        file_name: String,
        bytes: Vec<u8>,
        capture_method: &str,
    ) -> Result<SavedItem, VaultError> {
        let staging_root = self.root.join(HIDDEN_STATE_DIR).join("capture-staging");
        let staging = staging_root.join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&staging)?;
        let source_file = staging.join(readable_part(Some(&file_name), "captured-image"));
        let result = fs::write(&source_file, bytes)
            .map_err(VaultError::from)
            .and_then(|_| {
                self.preserve_artwork_item(
                    AddArtworkItem {
                        source_file,
                        home_subvault: "Paintings".to_string(),
                        creator: None,
                        year: None,
                        title,
                        saving_reason,
                    },
                    true,
                    &[],
                    Vec::new(),
                    Some(&source_link),
                    Some(capture_method),
                )
                .map(|outcome| outcome.saved_item)
            });
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_dir(&staging_root);
        if let Ok(saved) = &result {
            self.append_activity_log(&format!(
                "capture\t{}\tPaintings\t{capture_method}",
                saved.id()
            ))?;
        }
        result
    }

    fn capture_idea_source(&self, capture: IdeaSourceCapture) -> Result<SavedItem, VaultError> {
        if capture
            .copied_text
            .as_ref()
            .is_some_and(|text| text.len() as u64 + 1 > MAX_IDEA_SOURCE_TEXT_BYTES)
        {
            return Err(VaultError::IdeaSourceCopyTooLarge(PathBuf::from(
                "source-copies/cleaned-text.md",
            )));
        }
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
            &[],
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
        let scan = self.item_record_scan()?;
        let collection_search_text = self.collection_search_text_by_item_id()?;
        let hidden_state = self.root.join(HIDDEN_STATE_DIR);
        fs::create_dir_all(&hidden_state)?;

        let mut index = String::new();
        let mut indexed_items = 0;
        for record in scan.valid_records {
            let text = record.text;
            let id = required_frontmatter_value(&record.entry.record_path, &text, "id")?;
            let home_subvault =
                required_frontmatter_value(&record.entry.record_path, &text, "home_subvault")?;
            let title = required_frontmatter_value(&record.entry.record_path, &text, "title")?;
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
                &record.entry.item_folder.display().to_string(),
            ));
            index.push('\t');
            index.push_str(&escape_index_field(&title));
            index.push('\t');
            index.push_str(&escape_index_field(&searchable_text));
            index.push('\n');
            indexed_items += 1;
        }

        fs::write(hidden_state.join(METADATA_INDEX_FILE), index)?;
        self.append_activity_log(&format!("rebuild-metadata-index\t{indexed_items}"))?;

        Ok(RebuiltMetadataIndex {
            indexed_items,
            omitted_paths: scan
                .vault_problems
                .into_iter()
                .map(|problem| problem.path)
                .collect(),
        })
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
            let [id, home_subvault, item_folder, title, searchable_text] = fields.as_slice() else {
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
                title: title.to_string(),
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
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            let record = record.entry;
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
            let thumbnail = self.thumbnail_for_gallery(&saved_item)?;

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
                review_status: derived_review_status(&review_reasons(&text)).to_string(),
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

    pub fn prepare_thumbnail_previews(
        &self,
        home_subvault: &str,
        limit: usize,
    ) -> Result<ThumbnailPreparation, VaultError> {
        self.prepare_thumbnail_previews_with_failure_handler(
            home_subvault,
            limit,
            |id, primary_file, reason| {
                self.record_thumbnail_preview_failure(id, primary_file, reason)
            },
        )
    }

    pub fn prepare_thumbnail_previews_with_failure_handler<F>(
        &self,
        home_subvault: &str,
        limit: usize,
        mut record_failure: F,
    ) -> Result<ThumbnailPreparation, VaultError>
    where
        F: FnMut(&str, &Path, &str) -> Result<(), VaultError>,
    {
        let mut generated = 0;
        let mut remaining = 0;
        for record in self.item_record_scan()?.valid_records {
            if frontmatter_value(&record.text, "home_subvault").as_deref() != Some(home_subvault)
                || frontmatter_value(&record.text, "item_type").as_deref() != Some("artwork")
            {
                continue;
            }
            let saved_item = saved_item_from_record(&record.entry, &record.text)?;
            if self.thumbnail_is_final(&saved_item) {
                continue;
            }
            if generated >= limit {
                remaining += 1;
                continue;
            }
            let primary_file = record.entry.item_folder.join(required_frontmatter_value(
                &record.entry.record_path,
                &record.text,
                "primary_file",
            )?);
            self.generate_thumbnail_for(&saved_item, &primary_file, &mut record_failure)?;
            generated += 1;
        }
        Ok(ThumbnailPreparation {
            generated,
            remaining,
        })
    }

    pub fn prepare_thumbnail_preview(&self, id: &str) -> Result<ThumbnailPreparation, VaultError> {
        self.prepare_thumbnail_preview_with_failure_handler(id, |id, primary_file, reason| {
            self.record_thumbnail_preview_failure(id, primary_file, reason)
        })
    }

    pub fn prepare_thumbnail_preview_with_failure_handler<F>(
        &self,
        id: &str,
        mut record_failure: F,
    ) -> Result<ThumbnailPreparation, VaultError>
    where
        F: FnMut(&str, &Path, &str) -> Result<(), VaultError>,
    {
        let saved_item = self.open_saved_item(id)?;
        if self.thumbnail_is_final(&saved_item) {
            return Ok(ThumbnailPreparation {
                generated: 0,
                remaining: 0,
            });
        }
        let details = self.item_details(id)?;
        self.generate_thumbnail_for(
            &saved_item,
            details.primary_file(),
            &mut record_failure,
        )?;
        Ok(ThumbnailPreparation {
            generated: 1,
            remaining: 0,
        })
    }

    pub fn browse_idea_sources(&self) -> Result<Vec<IdeaSourceListItem>, VaultError> {
        let mut items = Vec::new();
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            let record = record.entry;
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
                review_status: derived_review_status(&review_reasons(&text)).to_string(),
                saving_reason: markdown_section(&text, "Saving Reason"),
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
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            let record = record.entry;
            let reasons = review_reasons(&text);
            let review_status = derived_review_status(&reasons).to_string();
            if review_status != "needs-review" {
                continue;
            }

            items.push(ReviewQueueItem {
                saved_item: saved_item_from_record(&record, &text)?,
                item_type: required_frontmatter_value(&record.record_path, &text, "item_type")?,
                title: required_frontmatter_value(&record.record_path, &text, "title")?,
                review_status,
                saving_reason: markdown_section(&text, "Saving Reason"),
                review_reasons: reasons,
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
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            let record = record.entry;
            if frontmatter_value(&text, "id").as_deref() != Some(id) {
                continue;
            }
            let item_type = required_frontmatter_value(&record.record_path, &text, "item_type")?;
            let title = required_frontmatter_value(&record.record_path, &text, "title")?;
            let creator = required_frontmatter_value(&record.record_path, &text, "creator")?;
            let year = required_frontmatter_value(&record.record_path, &text, "year")?;

            let mut links = item_links(&text);
            let trashed_ids: Vec<String> = self
                .trashed_item_record_scan()?
                .valid_records
                .into_iter()
                .filter_map(|record| frontmatter_value(&record.text, "id"))
                .collect();
            for link in &mut links {
                link.target_in_vault_trash = trashed_ids
                    .iter()
                    .any(|trashed_id| trashed_id == link.target());
            }
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
                review_status: derived_review_status(&review_reasons(&text)).to_string(),
                review_reasons: review_reasons(&text),
                tags: frontmatter_list(&text, "tags"),
                collections: self.collection_names_for_item(id, &text)?,
                item_links: links,
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
                folder_rename_proposal: folder_rename_proposal(
                    &record.item_folder,
                    &item_type,
                    &creator,
                    &year,
                    &title,
                ),
                import_original_filename: frontmatter_value(&text, "import_original_filename"),
                import_source_path: frontmatter_value(&text, "import_source_path")
                    .map(PathBuf::from),
                record_revision: item_record_revision(&text),
            });
        }

        Err(VaultError::SavedItemNotFound(id.to_string()))
    }

    pub fn read_idea_source(&self, id: &str) -> Result<IdeaSourceContent, VaultError> {
        let details = self.item_details(id)?;
        if details.home_subvault() != "Idea Sources" {
            return Err(VaultError::SavedItemNotFound(id.to_string()));
        }
        let source_copy = details
            .source_copy()
            .ok_or_else(|| VaultError::MissingIdeaSourceCopy(id.to_string()))?;
        let cleaned_text = read_bounded_idea_source(details.item_folder(), source_copy)?;
        Ok(IdeaSourceContent {
            id: id.to_string(),
            source_link: details.source_link().unwrap_or_default().to_string(),
            cleaned_text: cleaned_text.trim_end_matches('\n').to_string(),
            summary: details.summary().map(str::to_string),
        })
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

    pub fn save_item_record_edit(&self, edit: ItemRecordEdit) -> Result<ItemDetails, VaultError> {
        validate_item_record_edit(&edit)?;
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(edit.id.as_str()) {
                continue;
            }

            let actual_revision = item_record_revision(&text);
            if actual_revision != edit.expected_revision {
                return Err(VaultError::ItemRecordConflict {
                    path: record.record_path,
                    expected_revision: edit.expected_revision,
                    actual_revision,
                });
            }

            let mut registry = self.read_tag_registry()?;
            let mut tags = Vec::new();
            for input in &edit.tags {
                let canonical = canonical_tag(&registry, input);
                if !registry.iter().any(|tag| tag.name == canonical) {
                    registry.push(TagDefinition {
                        name: canonical.clone(),
                        aliases: Vec::new(),
                        meaning: None,
                    });
                }
                if !tags.contains(&canonical) {
                    tags.push(canonical);
                }
            }
            tags.sort();
            registry.sort_by(|left, right| left.name.cmp(&right.name));

            let mut updated = update_frontmatter_values(
                &text,
                &[
                    ("title", serde_yaml::Value::String(edit.title)),
                    ("creator", serde_yaml::Value::String(edit.creator)),
                    ("year", serde_yaml::Value::String(edit.year)),
                    (
                        "tags",
                        serde_yaml::Value::Sequence(
                            tags.into_iter().map(serde_yaml::Value::String).collect(),
                        ),
                    ),
                ],
            )?;
            updated = replace_or_append_markdown_section(
                &updated,
                "Saving Reason",
                edit.saving_reason.trim(),
            );
            updated = replace_or_append_markdown_section(&updated, "Summary", edit.summary.trim());

            atomic_write(&record.record_path, updated.as_bytes())?;
            self.write_tag_registry(&registry)?;
            self.rebuild_metadata_index()?;
            return self.item_details(&edit.id);
        }

        Err(VaultError::SavedItemNotFound(edit.id))
    }

    pub fn resolve_review_reason(
        &self,
        resolution: ReviewReasonResolution,
    ) -> Result<ItemDetails, VaultError> {
        for record in self.item_record_entries()? {
            let text = fs::read_to_string(&record.record_path)?;
            if frontmatter_value(&text, "id").as_deref() != Some(resolution.item_id.as_str()) {
                continue;
            }

            let actual_revision = item_record_revision(&text);
            if actual_revision != resolution.expected_revision {
                return Err(VaultError::ItemRecordConflict {
                    path: record.record_path,
                    expected_revision: resolution.expected_revision,
                    actual_revision,
                });
            }

            let mut reasons = review_reasons(&text);
            let reason_index = reasons
                .iter()
                .position(|reason| reason.id == resolution.reason_id)
                .ok_or_else(|| VaultError::ReviewReasonNotFound(resolution.reason_id.clone()))?;
            let reason = reasons.remove(reason_index);
            let mut updated = text;

            if let ReviewReasonAction::Correct { value } = &resolution.action {
                if value.trim().is_empty() {
                    return Err(VaultError::ReviewReasonCannotBeCorrected(reason.id));
                }
                match reason.target_field.as_deref() {
                    Some(field) if matches!(field, "title" | "creator" | "year") => {
                        updated = update_frontmatter_values(
                            &updated,
                            &[(field, serde_yaml::Value::String(value.trim().to_string()))],
                        )?;
                    }
                    _ => {
                        let mut corrections = markdown_list_section(&updated, "Review Resolutions");
                        corrections.push(format!(
                            "{} | corrected | {}",
                            reason.id,
                            value.trim().replace('|', "/")
                        ));
                        updated = replace_or_append_markdown_list_section(
                            &updated,
                            "Review Resolutions",
                            &corrections,
                        );
                    }
                }
            }

            if reason.kind == "metadata-suggestion" {
                let field = reason
                    .target_field
                    .as_deref()
                    .ok_or_else(|| VaultError::ReviewReasonCannotBeCorrected(reason.id.clone()))?;
                let mut suggestions = metadata_suggestions(&updated);
                let suggestion_index = suggestions
                    .iter()
                    .position(|suggestion| suggestion.field() == field)
                    .ok_or_else(|| VaultError::ReviewReasonNotFound(reason.id.clone()))?;
                let mut suggestion = suggestions.remove(suggestion_index);

                match &resolution.action {
                    ReviewReasonAction::Accept => {
                        updated = update_frontmatter_values(
                            &updated,
                            &[(
                                field,
                                serde_yaml::Value::String(suggestion.suggested_value().to_string()),
                            )],
                        )?;
                    }
                    ReviewReasonAction::Correct { value } => {
                        suggestion.provenance = format!(
                            "{}; edited during review from {}",
                            suggestion.provenance, suggestion.suggested_value
                        );
                        suggestion.suggested_value = value.trim().to_string();
                    }
                    ReviewReasonAction::Dismiss => {}
                }

                if !matches!(resolution.action, ReviewReasonAction::Dismiss) {
                    let mut provenance = metadata_provenance(&updated);
                    provenance.push(suggestion);
                    let lines = provenance
                        .iter()
                        .map(metadata_suggestion_line)
                        .collect::<Vec<_>>();
                    updated = replace_or_append_markdown_list_section(
                        &updated,
                        "Metadata Provenance",
                        &lines,
                    );
                }
                let suggestion_lines = suggestions
                    .iter()
                    .map(metadata_suggestion_line)
                    .collect::<Vec<_>>();
                updated = replace_or_append_markdown_list_section(
                    &updated,
                    "Metadata Suggestions",
                    &suggestion_lines,
                );
            }

            updated = update_frontmatter_values(
                &updated,
                &[
                    ("review_reasons", review_reason_sequence(&reasons)),
                    (
                        "review_status",
                        serde_yaml::Value::String(derived_review_status(&reasons).to_string()),
                    ),
                ],
            )?;
            atomic_write(&record.record_path, updated.as_bytes())?;
            self.rebuild_metadata_index()?;
            return self.item_details(&resolution.item_id);
        }

        Err(VaultError::SavedItemNotFound(resolution.item_id))
    }

    pub fn resolve_duplicate_candidate(
        &self,
        resolution: DuplicateCandidateResolution,
    ) -> Result<DuplicateCandidateResolutionOutcome, VaultError> {
        let move_to_trash = resolution.action == DuplicateCandidateAction::MoveThisItemToVaultTrash;
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            if frontmatter_value(&text, "id").as_deref() != Some(resolution.item_id.as_str()) {
                continue;
            }
            let actual_revision = item_record_revision(&text);
            if actual_revision != resolution.expected_revision {
                return Err(VaultError::ItemRecordConflict {
                    path: record.entry.record_path,
                    expected_revision: resolution.expected_revision,
                    actual_revision,
                });
            }
            let mut reasons = review_reasons(&text);
            let reason_index = reasons
                .iter()
                .position(|reason| {
                    reason.id == resolution.reason_id && reason.kind == "duplicate-candidate"
                })
                .ok_or_else(|| VaultError::ReviewReasonNotFound(resolution.reason_id.clone()))?;
            reasons.remove(reason_index);
            let decision = match resolution.action {
                DuplicateCandidateAction::NotADuplicate => "not-a-duplicate",
                DuplicateCandidateAction::KeepBoth => "keep-both",
                DuplicateCandidateAction::MoveThisItemToVaultTrash => {
                    "moved-this-item-to-vault-trash"
                }
            };
            let mut decisions = markdown_list_section(&text, "Duplicate Candidate Decisions");
            decisions.push(format!("{} | {decision}", resolution.reason_id));
            let updated = replace_or_append_markdown_list_section(
                &update_frontmatter_values(
                    &text,
                    &[
                        ("review_reasons", review_reason_sequence(&reasons)),
                        (
                            "review_status",
                            serde_yaml::Value::String(derived_review_status(&reasons).to_string()),
                        ),
                    ],
                )?,
                "Duplicate Candidate Decisions",
                &decisions,
            );
            return if move_to_trash {
                let original_folder = record.entry.item_folder.clone();
                let item = self.move_item_to_trash(&resolution.item_id)?;
                if let Err(write_error) =
                    atomic_write(&item.item_folder().join("record.md"), updated.as_bytes())
                {
                    fs::rename(item.item_folder(), &original_folder)?;
                    self.rebuild_metadata_index()?;
                    self.append_activity_log(&format!(
                        "trash-item-rollback\t{}\t{}",
                        resolution.item_id,
                        original_folder.display()
                    ))?;
                    return Err(write_error);
                }
                Ok(DuplicateCandidateResolutionOutcome::MovedToVaultTrash(item))
            } else {
                atomic_write(&record.entry.record_path, updated.as_bytes())?;
                self.rebuild_metadata_index()?;
                self.item_details(&resolution.item_id)
                    .map(DuplicateCandidateResolutionOutcome::Active)
            };
        }
        Err(VaultError::SavedItemNotFound(resolution.item_id))
    }

    pub fn confirm_item_folder_rename(
        &self,
        id: &str,
        reviewed_proposal: &ItemFolderRenameProposal,
    ) -> Result<ItemDetails, VaultError> {
        let details = self.item_details(id)?;
        let current_proposal = details
            .folder_rename_proposal()
            .ok_or_else(|| VaultError::NoItemFolderRenameSuggested(id.to_string()))?;
        if current_proposal != reviewed_proposal || reviewed_proposal.proposed_path.exists() {
            return Err(VaultError::ItemFolderRenameProposalChanged(id.to_string()));
        }
        fs::rename(
            &reviewed_proposal.current_path,
            &reviewed_proposal.proposed_path,
        )?;
        self.rebuild_metadata_index()?;
        self.item_details(id)
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
            .map(|path| read_bounded_idea_source(details.item_folder(), path))
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

        self.apply_ai_enrichment_response(
            id,
            None,
            budget_mode,
            response,
            provider.cost_estimate_is_known(),
        )
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

        self.apply_ai_enrichment_response(
            id,
            None,
            budget_mode,
            response,
            provider.cost_estimate_is_known(),
        )
    }

    pub fn apply_artwork_enrichment_response(
        &self,
        id: &str,
        expected_revision: &str,
        budget_mode: AiBudgetMode,
        response: AiProviderResponse,
        cost_estimate_is_known: bool,
    ) -> Result<AiEnrichmentResult, VaultError> {
        self.apply_ai_enrichment_response(
            id,
            Some(expected_revision),
            budget_mode,
            response,
            cost_estimate_is_known,
        )
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
        for record in self.item_record_scan()?.valid_records {
            let text = record.text;
            let record = record.entry;
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

    fn item_record_scan(&self) -> Result<ItemRecordScan, VaultError> {
        Ok(scan_item_record_entries(self.item_record_entries()?))
    }

    fn trashed_item_record_scan(&self) -> Result<ItemRecordScan, VaultError> {
        Ok(scan_item_record_entries(
            self.trashed_item_record_entries()?,
        ))
    }

    fn trashed_item_record_entries(&self) -> Result<Vec<ItemRecordEntry>, VaultError> {
        let mut entries = Vec::new();
        let trash = self.root.join(TRASH_DIR);
        if trash.is_dir() {
            for home in fs::read_dir(trash)? {
                let home = home?;
                if !home.file_type()?.is_dir() {
                    continue;
                }
                for folder in fs::read_dir(home.path())? {
                    let folder = folder?;
                    let record_path = folder.path().join("record.md");
                    if folder.file_type()?.is_dir() && record_path.is_file() {
                        entries.push(ItemRecordEntry {
                            record_path,
                            item_folder: folder.path(),
                        });
                    }
                }
            }
        }
        entries.sort_by(|left, right| left.item_folder.cmp(&right.item_folder));
        Ok(entries)
    }

    fn item_record_by_id(
        &self,
        entries: Vec<ItemRecordEntry>,
        id: &str,
        error: VaultError,
    ) -> Result<ValidItemRecord, VaultError> {
        scan_item_record_entries(entries)
            .valid_records
            .into_iter()
            .find(|record| frontmatter_value(&record.text, "id").as_deref() == Some(id))
            .ok_or(error)
    }

    fn find_active_record(&self, id: &str) -> Result<ValidItemRecord, VaultError> {
        self.item_record_by_id(
            self.item_record_entries()?,
            id,
            VaultError::SavedItemNotFound(id.to_string()),
        )
    }

    fn find_trashed_record(&self, id: &str) -> Result<ValidItemRecord, VaultError> {
        self.item_record_by_id(
            self.trashed_item_record_entries()?,
            id,
            VaultError::TrashedItemNotFound(id.to_string()),
        )
    }

    fn incoming_item_links(&self, id: &str) -> Result<Vec<IncomingItemLink>, VaultError> {
        let mut links = Vec::new();
        let records = self
            .item_record_scan()?
            .valid_records
            .into_iter()
            .chain(self.trashed_item_record_scan()?.valid_records);
        for record in records {
            for link in item_links(&record.text)
                .into_iter()
                .filter(|link| link.target() == id)
            {
                links.push(IncomingItemLink {
                    source_item_id: required_frontmatter_value(
                        &record.entry.record_path,
                        &record.text,
                        "id",
                    )?,
                    label: link.label().to_string(),
                });
            }
        }
        Ok(links)
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
        excluded_candidate_ids: &[String],
    ) -> Result<Vec<DuplicateCandidate>, VaultError> {
        let mut candidates = Vec::new();
        for record in self.item_record_entries()? {
            let Ok(text) = fs::read_to_string(&record.record_path) else {
                continue;
            };
            let Some(id) = frontmatter_value(&text, "id") else {
                continue;
            };
            if excluded_candidate_ids
                .iter()
                .any(|excluded| excluded == &id)
            {
                continue;
            }

            if let Some(file_fingerprint) = file_fingerprint {
                if frontmatter_value(&text, "file_fingerprint").as_deref() == Some(file_fingerprint)
                {
                    push_duplicate_candidate(&mut candidates, id.clone(), "file-fingerprint");
                }
            }

            if let Some(source_path) = source_path {
                if source_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    == frontmatter_value(&text, "import_original_filename")
                {
                    push_duplicate_candidate(&mut candidates, id.clone(), "filename");
                }
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
        let mut existing_item_ids = Vec::new();
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
            let Some(item_type) = frontmatter_value(&text, "item_type") else {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "missing item type".to_string(),
                });
                continue;
            };
            if item_type != "artwork" {
                continue;
            }
            let Some(id) = frontmatter_value(&text, "id") else {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "missing item id".to_string(),
                });
                continue;
            };
            let Some(stored_fingerprint) = frontmatter_value(&text, "file_fingerprint") else {
                vault_problems.push(ImportVaultProblem {
                    path: record.record_path.clone(),
                    error: "missing file fingerprint".to_string(),
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
            if stored_fingerprint != incoming_fingerprint {
                continue;
            }
            match fs::read(&preserved_path) {
                Ok(existing_bytes) if existing_bytes == incoming_bytes => {
                    existing_item_ids.push(id);
                }
                Ok(_) => {}
                Err(error) => vault_problems.push(ImportVaultProblem {
                    path: preserved_path,
                    error: error.to_string(),
                }),
            }
        }
        Ok(ExactDuplicateCheck {
            existing_item_ids,
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

    fn thumbnail_for_gallery(
        &self,
        saved_item: &SavedItem,
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

        let pending_file = thumbnails_dir.join("pending.png");
        if !pending_file.is_file() {
            write_thumbnail_placeholder(&pending_file)?;
        }
        Ok(ThumbnailPreview {
            path: pending_file,
            is_placeholder: true,
        })
    }

    fn thumbnail_is_final(&self, saved_item: &SavedItem) -> bool {
        let thumbnails_dir = self.root.join(HIDDEN_STATE_DIR).join(THUMBNAILS_DIR);
        thumbnails_dir
            .join(format!("{}.png", saved_item.id()))
            .is_file()
            || thumbnails_dir
                .join(format!("{}.placeholder.png", saved_item.id()))
                .is_file()
    }

    fn generate_thumbnail_for<F>(
        &self,
        saved_item: &SavedItem,
        primary_file: &Path,
        record_failure: &mut F,
    ) -> Result<ThumbnailPreview, VaultError>
    where
        F: FnMut(&str, &Path, &str) -> Result<(), VaultError>,
    {
        let thumbnails_dir = self.root.join(HIDDEN_STATE_DIR).join(THUMBNAILS_DIR);
        fs::create_dir_all(&thumbnails_dir)?;
        let thumbnail_file = thumbnails_dir.join(format!("{}.png", saved_item.id()));
        let placeholder_file = thumbnails_dir.join(format!("{}.placeholder.png", saved_item.id()));

        let dimensions = fast_webp_dimensions(primary_file)
            .map(Ok)
            .unwrap_or_else(|| {
                image::image_dimensions(primary_file).map_err(|error| error.to_string())
            });
        let decoded = dimensions.and_then(|(width, height)| {
            if !thumbnail_dimensions_are_safe(width, height) {
                return Err(format!(
                    "image dimensions {width}x{height} exceed the safe Thumbnail Preview limit"
                ));
            }
            image::io::Reader::open(primary_file)
                .map_err(|error| error.to_string())?
                .with_guessed_format()
                .map_err(|error| error.to_string())?
                .decode()
                .map_err(|error| error.to_string())
        });
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
                record_failure(saved_item.id(), primary_file, &reason)?;
                write_thumbnail_placeholder(&placeholder_file)?;
                Ok(ThumbnailPreview {
                    path: placeholder_file,
                    is_placeholder: true,
                })
            }
        }
    }

    pub fn record_thumbnail_preview_failure(
        &self,
        id: &str,
        primary_file: &Path,
        reason: &str,
    ) -> Result<(), VaultError> {
        let saved_item = self.open_saved_item(id)?;
        let record_path = saved_item.item_folder.join("record.md");
        let record = fs::read_to_string(&record_path)?;
        let review_reason = ReviewReason {
            id: "thumbnail-preview-unavailable".to_string(),
            kind: "thumbnail-preview-unavailable".to_string(),
            target_field: None,
            message: "Thumbnail Preview is unavailable".to_string(),
            evidence: reason.to_string(),
            candidate_item_id: None,
        };
        if review_reasons(&record)
            .iter()
            .any(|existing| existing.kind == "thumbnail-preview-unavailable")
        {
            return Ok(());
        }

        let mut review_reasons = frontmatter_list(&record, "review_reasons");
        review_reasons.push(review_reason_line(&review_reason));
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
        expected_revision: Option<&str>,
        budget_mode: AiBudgetMode,
        response: AiProviderResponse,
        cost_estimate_is_known: bool,
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

            if let Some(expected_revision) = expected_revision {
                let actual_revision = item_record_revision(&text);
                if actual_revision != expected_revision {
                    return Err(VaultError::ItemRecordConflict {
                        path: record.record_path,
                        expected_revision: expected_revision.to_string(),
                        actual_revision,
                    });
                }
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
                let reasons = review_reasons(&updated)
                    .into_iter()
                    .filter(|reason| {
                        !accepted_metadata.iter().any(|suggestion| {
                            reason.kind == "unknown-metadata"
                                && reason.target_field.as_deref() == Some(suggestion.field())
                        })
                    })
                    .collect::<Vec<_>>();
                updated = update_frontmatter_values(
                    &updated,
                    &[
                        ("review_reasons", review_reason_sequence(&reasons)),
                        (
                            "review_status",
                            serde_yaml::Value::String(derived_review_status(&reasons).to_string()),
                        ),
                    ],
                )?;
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
                let mut reasons = review_reasons(&updated);
                for suggestion in &staged_suggestions {
                    let id = format!("metadata-suggestion-{}", suggestion.field());
                    if reasons.iter().any(|reason| reason.id == id) {
                        continue;
                    }
                    reasons.push(ReviewReason {
                        id,
                        kind: "metadata-suggestion".to_string(),
                        target_field: Some(suggestion.field().to_string()),
                        message: format!(
                            "Suggested {}: {}",
                            suggestion.field(),
                            suggestion.suggested_value()
                        ),
                        evidence: suggestion.provenance().to_string(),
                        candidate_item_id: None,
                    });
                }
                updated = update_frontmatter_values(
                    &updated,
                    &[
                        ("review_reasons", review_reason_sequence(&reasons)),
                        (
                            "review_status",
                            serde_yaml::Value::String(derived_review_status(&reasons).to_string()),
                        ),
                    ],
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
            if cost_estimate_is_known {
                self.append_ai_cost_log(id, budget_mode, estimated_cost_cents)?;
                self.append_activity_log(&format!(
                    "ai-cost\t{}\t{}\t{}",
                    id,
                    budget_mode.as_log_value(),
                    estimated_cost_cents
                ))?;
            }
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

fn thumbnail_dimensions_are_safe(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) <= MAX_THUMBNAIL_SOURCE_PIXELS
}

fn fast_webp_dimensions(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut header = [0_u8; 30];
    fs::File::open(path).ok()?.read_exact(&mut header).ok()?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WEBP" {
        return None;
    }
    match &header[12..16] {
        b"VP8 " if header[23..26] == [0x9d, 0x01, 0x2a] => Some((
            u32::from(u16::from_le_bytes([header[26], header[27]]) & 0x3fff),
            u32::from(u16::from_le_bytes([header[28], header[29]]) & 0x3fff),
        )),
        b"VP8L" if header[20] == 0x2f => Some((
            1 + u32::from(header[21]) + (u32::from(header[22] & 0x3f) << 8),
            1 + (u32::from(header[22] >> 6)
                | (u32::from(header[23]) << 2)
                | (u32::from(header[24] & 0x0f) << 10)),
        )),
        b"VP8X" => Some((
            1 + u32::from(header[24])
                + (u32::from(header[25]) << 8)
                + (u32::from(header[26]) << 16),
            1 + u32::from(header[27])
                + (u32::from(header[28]) << 8)
                + (u32::from(header[29]) << 16),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod thumbnail_dimension_tests {
    use std::fs;

    use super::{fast_webp_dimensions, thumbnail_dimensions_are_safe};

    #[test]
    fn rejects_reported_8000_square_source_before_full_decode() {
        assert!(!thumbnail_dimensions_are_safe(8_000, 8_000));
        assert!(thumbnail_dimensions_are_safe(6_000, 4_000));
    }

    #[test]
    fn reads_vp8_dimensions_from_the_small_header_only() {
        let path = std::env::temp_dir().join(format!("webp-header-{}.webp", std::process::id()));
        let mut header = [0_u8; 30];
        header[0..4].copy_from_slice(b"RIFF");
        header[8..12].copy_from_slice(b"WEBP");
        header[12..16].copy_from_slice(b"VP8 ");
        header[23..26].copy_from_slice(&[0x9d, 0x01, 0x2a]);
        header[26..28].copy_from_slice(&8_000_u16.to_le_bytes());
        header[28..30].copy_from_slice(&8_000_u16.to_le_bytes());
        fs::write(&path, header).expect("write WebP header fixture");
        assert_eq!(fast_webp_dimensions(&path), Some((8_000, 8_000)));
        fs::remove_file(path).expect("clean WebP header fixture");
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailPreparation {
    generated: usize,
    remaining: usize,
}

impl ThumbnailPreparation {
    pub fn generated(&self) -> usize {
        self.generated
    }

    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreservedArtwork {
    saved_item: SavedItem,
    duplicate_candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactDuplicateCheck {
    existing_item_ids: Vec<String>,
    vault_problems: Vec<ImportVaultProblem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportActivityKind {
    FolderRun,
    SelectedFiles,
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
    saving_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeaSourceContent {
    id: String,
    source_link: String,
    cleaned_text: String,
    summary: Option<String>,
}

impl IdeaSourceContent {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn source_link(&self) -> &str {
        &self.source_link
    }
    pub fn cleaned_text(&self) -> &str {
        &self.cleaned_text
    }
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
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

    pub fn saving_reason(&self) -> Option<&str> {
        self.saving_reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueItem {
    saved_item: SavedItem,
    item_type: String,
    title: String,
    review_status: String,
    saving_reason: Option<String>,
    review_reasons: Vec<ReviewReason>,
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

    pub fn saving_reason(&self) -> Option<&str> {
        self.saving_reason.as_deref()
    }

    pub fn review_reasons(&self) -> &[ReviewReason] {
        &self.review_reasons
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewReason {
    id: String,
    kind: String,
    target_field: Option<String>,
    message: String,
    evidence: String,
    candidate_item_id: Option<String>,
}

impl ReviewReason {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn target_field(&self) -> Option<&str> {
        self.target_field.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn evidence(&self) -> &str {
        &self.evidence
    }

    pub fn candidate_item_id(&self) -> Option<&str> {
        self.candidate_item_id.as_deref()
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
    review_reasons: Vec<ReviewReason>,
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
    folder_rename_proposal: Option<ItemFolderRenameProposal>,
    import_original_filename: Option<String>,
    import_source_path: Option<PathBuf>,
    record_revision: String,
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

    pub fn review_reasons(&self) -> &[ReviewReason] {
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

    pub fn folder_rename_proposal(&self) -> Option<&ItemFolderRenameProposal> {
        self.folder_rename_proposal.as_ref()
    }

    pub fn folder_rename_suggestion(&self) -> Option<&str> {
        self.folder_rename_proposal()?
            .proposed_path()
            .file_name()?
            .to_str()
    }

    pub fn import_original_filename(&self) -> Option<&str> {
        self.import_original_filename.as_deref()
    }

    pub fn import_source_path(&self) -> Option<&Path> {
        self.import_source_path.as_deref()
    }

    pub fn record_revision(&self) -> &str {
        &self.record_revision
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFolderRenameProposal {
    current_path: PathBuf,
    proposed_path: PathBuf,
}

impl ItemFolderRenameProposal {
    pub fn reviewed(current_path: impl Into<PathBuf>, proposed_path: impl Into<PathBuf>) -> Self {
        Self {
            current_path: current_path.into(),
            proposed_path: proposed_path.into(),
        }
    }

    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    pub fn proposed_path(&self) -> &Path {
        &self.proposed_path
    }
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
pub struct ArtworkImportOptions {
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
pub struct ArtworkImportOutcome {
    imported_items: Vec<SavedItem>,
    skipped_entries: Vec<ImportSkippedEntry>,
    failed_entries: Vec<ImportFailedEntry>,
    duplicate_candidate_entries: Vec<ImportDuplicateCandidateEntry>,
    cancelled_files: Vec<PathBuf>,
    maintenance_errors: Vec<String>,
    vault_problems: Vec<ImportVaultProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRunSummary(ArtworkImportOutcome);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFileImportSummary(ArtworkImportOutcome);

impl std::ops::Deref for ImportRunSummary {
    type Target = ArtworkImportOutcome;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for SelectedFileImportSummary {
    type Target = ArtworkImportOutcome;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SelectedFileImportSummary {
    pub fn into_outcome(self) -> ArtworkImportOutcome {
        self.0
    }
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

impl ArtworkImportOutcome {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRecordEdit {
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
pub struct ReviewReasonResolution {
    pub item_id: String,
    pub reason_id: String,
    pub expected_revision: String,
    pub action: ReviewReasonAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewReasonAction {
    Accept,
    Correct { value: String },
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateCandidateAction {
    NotADuplicate,
    KeepBoth,
    MoveThisItemToVaultTrash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateCandidateResolution {
    pub item_id: String,
    pub reason_id: String,
    pub expected_revision: String,
    pub action: DuplicateCandidateAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DuplicateCandidateResolutionOutcome {
    Active(ItemDetails),
    MovedToVaultTrash(SavedItem),
}

impl DuplicateCandidateResolutionOutcome {
    pub fn active_item(&self) -> Option<&ItemDetails> {
        match self {
            Self::Active(item) => Some(item),
            Self::MovedToVaultTrash(_) => None,
        }
    }

    pub fn trashed_item(&self) -> Option<&SavedItem> {
        match self {
            Self::Active(_) => None,
            Self::MovedToVaultTrash(item) => Some(item),
        }
    }
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
    target_in_vault_trash: bool,
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
    pub fn target_in_vault_trash(&self) -> bool {
        self.target_in_vault_trash
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
    fn cost_estimate_is_known(&self) -> bool {
        true
    }
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
    ExtractedImage {
        title: Option<String>,
        file_name: String,
        bytes: Vec<u8>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedItem {
    saved_item: SavedItem,
    title: String,
    creator: String,
    year: String,
    review_status: String,
    tags: Vec<String>,
    collections: Vec<String>,
    incoming_item_links: Vec<IncomingItemLink>,
}

impl TrashedItem {
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
    pub fn review_status(&self) -> &str {
        &self.review_status
    }
    pub fn tags(&self) -> Vec<&str> {
        self.tags.iter().map(String::as_str).collect()
    }
    pub fn collections(&self) -> Vec<&str> {
        self.collections.iter().map(String::as_str).collect()
    }
    pub fn incoming_item_links(&self) -> &[IncomingItemLink] {
        &self.incoming_item_links
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingItemLink {
    source_item_id: String,
    label: String,
}

impl IncomingItemLink {
    pub fn source_item_id(&self) -> &str {
        &self.source_item_id
    }
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermanentDeletion {
    id: String,
    collections: Vec<String>,
    incoming_item_links: Vec<IncomingItemLink>,
}

impl PermanentDeletion {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn collections(&self) -> &[String] {
        &self.collections
    }
    pub fn incoming_item_links(&self) -> &[IncomingItemLink] {
        &self.incoming_item_links
    }
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
    omitted_paths: Vec<PathBuf>,
}

impl RebuiltMetadataIndex {
    pub fn indexed_items(&self) -> usize {
        self.indexed_items
    }

    pub fn omitted_paths(&self) -> &[PathBuf] {
        &self.omitted_paths
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultProblem {
    path: PathBuf,
    error: String,
}

impl VaultProblem {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    saved_item: SavedItem,
    title: String,
}

impl SearchResult {
    pub fn saved_item(&self) -> &SavedItem {
        &self.saved_item
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemRecordEntry {
    record_path: PathBuf,
    item_folder: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidItemRecord {
    entry: ItemRecordEntry,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemRecordScan {
    valid_records: Vec<ValidItemRecord>,
    vault_problems: Vec<VaultProblem>,
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
    MalformedConfig {
        path: PathBuf,
        reason: String,
    },
    UnsupportedFormat(PathBuf),
    MissingSourceFile(PathBuf),
    MissingIdeaSourceCopy(String),
    UnsafeIdeaSourceCopy(PathBuf),
    EmptyIdeaSourceCopy(PathBuf),
    IdeaSourceCopyTooLarge(PathBuf),
    MissingFileName(PathBuf),
    UnsupportedImageFile(PathBuf),
    MissingImportFolder(PathBuf),
    SavedItemNotFound(String),
    TrashedItemNotFound(String),
    PermanentDeletionNotConfirmed,
    ReferenceCleanupBlocked(PathBuf),
    ReferenceCleanupRollbackFailed(PathBuf),
    CollectionNotFound(String),
    MalformedItemRecord(PathBuf),
    ItemRecordCodec(String),
    PreviewGeneration(String),
    InvalidItemRecordEdit(Vec<String>),
    ReviewReasonNotFound(String),
    ReviewReasonCannotBeCorrected(String),
    ItemRecordConflict {
        path: PathBuf,
        expected_revision: String,
        actual_revision: String,
    },
    NoItemFolderRenameSuggested(String),
    ItemFolderRenameProposalChanged(String),
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
            Self::MissingIdeaSourceCopy(id) => {
                write!(f, "Idea Source has no preserved readable source copy: {id}")
            }
            Self::UnsafeIdeaSourceCopy(path) => write!(
                f,
                "Idea Source copy is outside its item folder: {}",
                path.display()
            ),
            Self::EmptyIdeaSourceCopy(path) => {
                write!(f, "Idea Source copy is empty: {}", path.display())
            }
            Self::IdeaSourceCopyTooLarge(path) => write!(
                f,
                "Idea Source copy exceeds the 2 MiB read limit: {}",
                path.display()
            ),
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
            Self::TrashedItemNotFound(id) => write!(f, "trashed item not found: {id}"),
            Self::PermanentDeletionNotConfirmed => write!(
                f,
                "permanent deletion requires confirmation of the exact item id"
            ),
            Self::ReferenceCleanupBlocked(path) => write!(
                f,
                "reference cleanup is blocked by malformed item record: {}",
                path.display()
            ),
            Self::ReferenceCleanupRollbackFailed(path) => write!(
                f,
                "reference cleanup rollback failed for: {}",
                path.display()
            ),
            Self::CollectionNotFound(id) => write!(f, "collection not found: {id}"),
            Self::MalformedItemRecord(path) => {
                write!(f, "item record is malformed: {}", path.display())
            }
            Self::ItemRecordCodec(reason) => write!(f, "item record codec failed: {reason}"),
            Self::PreviewGeneration(reason) => {
                write!(f, "thumbnail preview generation failed: {reason}")
            }
            Self::InvalidItemRecordEdit(reasons) => {
                write!(f, "item record edit is invalid: {}", reasons.join("; "))
            }
            Self::ReviewReasonNotFound(id) => write!(f, "review reason not found: {id}"),
            Self::ReviewReasonCannotBeCorrected(id) => {
                write!(
                    f,
                    "review reason cannot be corrected with a field value: {id}"
                )
            }
            Self::ItemRecordConflict { path, .. } => {
                write!(
                    f,
                    "item record changed after the editor loaded it: {}",
                    path.display()
                )
            }
            Self::NoItemFolderRenameSuggested(id) => {
                write!(f, "item folder rename is not suggested: {id}")
            }
            Self::ItemFolderRenameProposalChanged(id) => {
                write!(f, "item folder rename proposal changed: {id}")
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

fn read_bounded_idea_source(item_folder: &Path, source_copy: &Path) -> Result<String, VaultError> {
    if source_copy.is_absolute()
        || source_copy.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(VaultError::UnsafeIdeaSourceCopy(source_copy.to_path_buf()));
    }
    let canonical_item = item_folder.canonicalize()?;
    let canonical_source_copies = item_folder.join("source-copies").canonicalize()?;
    let path = item_folder.join(source_copy);
    let canonical_source = path.canonicalize()?;
    if !canonical_source.starts_with(&canonical_item)
        || !canonical_source.starts_with(&canonical_source_copies)
        || !canonical_source.is_file()
    {
        return Err(VaultError::UnsafeIdeaSourceCopy(path));
    }
    if canonical_source.metadata()?.len() > MAX_IDEA_SOURCE_TEXT_BYTES {
        return Err(VaultError::IdeaSourceCopyTooLarge(canonical_source));
    }
    let mut bytes = Vec::new();
    fs::File::open(&canonical_source)?
        .take(MAX_IDEA_SOURCE_TEXT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IDEA_SOURCE_TEXT_BYTES {
        return Err(VaultError::IdeaSourceCopyTooLarge(canonical_source));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| VaultError::UnsafeIdeaSourceCopy(canonical_source.clone()))?;
    if text.trim().is_empty() {
        return Err(VaultError::EmptyIdeaSourceCopy(canonical_source));
    }
    Ok(text)
}

fn default_config() -> String {
    format!("format_version = 2\nname = \"{DEFAULT_VAULT_NAME}\"\n")
}

fn validate_item_record_edit(edit: &ItemRecordEdit) -> Result<(), VaultError> {
    let mut reasons = Vec::new();
    if edit.title.trim().is_empty() {
        reasons.push("title must not be empty".to_string());
    }
    if edit.creator.trim().is_empty() {
        reasons.push("creator must not be empty".to_string());
    }
    let year = edit.year.trim();
    if year.is_empty() {
        reasons.push("year must not be empty".to_string());
    } else if year != "Unknown Year"
        && (year.len() != 4 || !year.chars().all(|character| character.is_ascii_digit()))
    {
        reasons.push("year must be four digits or Unknown Year".to_string());
    }
    if edit.tags.iter().any(|tag| tag.trim().is_empty()) {
        reasons.push("tags must not contain empty values".to_string());
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(VaultError::InvalidItemRecordEdit(reasons))
    }
}

fn item_record_revision(record: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in record.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), VaultError> {
    let parent = path.parent().ok_or_else(|| {
        VaultError::ItemRecordCodec("item record has no parent directory".to_string())
    })?;
    let temporary = parent.join(format!(".record-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<(), VaultError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn artwork_folder_name(creator: Option<&str>, year: Option<&str>, title: &str) -> String {
    format!(
        "{} - {} - {}",
        readable_part(creator, "Unknown Creator"),
        readable_part(year, "Unknown Year"),
        readable_part(Some(title), "Untitled")
    )
}

fn folder_rename_proposal(
    item_folder: &Path,
    item_type: &str,
    creator: &str,
    year: &str,
    title: &str,
) -> Option<ItemFolderRenameProposal> {
    if item_type != "artwork" {
        return None;
    }

    let suggested = artwork_folder_name(Some(creator), Some(year), title);
    let current = item_folder.file_name()?.to_string_lossy();

    if suggested == current || is_collision_suffixed_name(&current, &suggested) {
        return None;
    }

    let parent = item_folder.parent()?;
    Some(ItemFolderRenameProposal {
        current_path: item_folder.to_path_buf(),
        proposed_path: unique_folder_path(parent, &suggested),
    })
}

fn is_collision_suffixed_name(current: &str, base: &str) -> bool {
    let Some(suffix) = current
        .strip_prefix(base)
        .and_then(|value| value.strip_prefix(" ("))
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    suffix.parse::<usize>().is_ok_and(|number| number >= 2)
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

fn inferred_artwork_item(
    source_file: PathBuf,
    metadata: &ArtworkImportMetadata,
) -> (AddArtworkItem, Vec<ReviewReason>) {
    let inferred = infer_artwork_metadata(&source_file);
    let mut conflicts = Vec::new();
    if let (Some(supplied), Some(from_filename)) = (&metadata.creator, &inferred.creator) {
        if supplied != from_filename {
            conflicts.push(metadata_conflict_review_reason(
                "creator",
                supplied,
                from_filename,
            ));
        }
    }
    if let (Some(supplied), Some(from_filename)) = (&metadata.year, &inferred.year) {
        if supplied != from_filename {
            conflicts.push(metadata_conflict_review_reason(
                "year",
                supplied,
                from_filename,
            ));
        }
    }
    (
        AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: metadata.creator.clone().or(inferred.creator),
            year: metadata.year.clone().or(inferred.year),
            title: inferred.title,
            saving_reason: metadata.saving_reason.clone(),
        },
        conflicts,
    )
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
    #[serde(skip_serializing_if = "Option::is_none")]
    source_link: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capture_method: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_original_filename: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_source_folder: Option<String>,
    // Gallery newest-sort reads this as added_at for every artwork item, including Source Link captures.
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
    additional_review_reasons: &[ReviewReason],
    source_link: Option<&str>,
    capture_method: Option<&str>,
) -> Result<String, VaultError> {
    let creator = item.creator.as_deref().unwrap_or("Unknown Creator");
    let year = item.year.as_deref().unwrap_or("Unknown Year");
    let saving_reason = item.saving_reason.as_deref().unwrap_or("");
    let review_status = if duplicate_candidates.is_empty()
        && additional_review_reasons.is_empty()
        && creator != "Unknown Creator"
        && year != "Unknown Year"
    {
        "reviewed"
    } else {
        "needs-review"
    };
    let (import_original_filename, import_source_path, import_source_folder) =
        if source_link.is_some() {
            (None, None, None)
        } else {
            (
                Some(import_original_filename),
                Some(import_source_path.display().to_string()),
                Some(
                    import_source_path
                        .parent()
                        .map(Path::display)
                        .map(|display| display.to_string())
                        .unwrap_or_default(),
                ),
            )
        };

    let frontmatter = ArtworkFrontmatter {
        id,
        item_type: "artwork",
        home_subvault: &item.home_subvault,
        title: &item.title,
        creator,
        year,
        primary_file,
        source_link,
        capture_method,
        import_original_filename,
        import_source_path,
        import_source_folder,
        imported_at,
        file_fingerprint,
        duplicate_candidates: duplicate_candidates
            .iter()
            .map(|candidate| format!("{} | {}", candidate.item_id, candidate.signal))
            .collect(),
        review_reasons: {
            let mut reasons = Vec::new();
            if creator == "Unknown Creator" {
                reasons.push(review_reason_line(&ReviewReason {
                    id: "unknown-creator".to_string(),
                    kind: "unknown-metadata".to_string(),
                    target_field: Some("creator".to_string()),
                    message: "Creator is unknown".to_string(),
                    evidence: "No creator metadata was supplied or inferred".to_string(),
                    candidate_item_id: None,
                }));
            }
            if year == "Unknown Year" {
                reasons.push(review_reason_line(&ReviewReason {
                    id: "unknown-year".to_string(),
                    kind: "unknown-metadata".to_string(),
                    target_field: Some("year".to_string()),
                    message: "Year is unknown".to_string(),
                    evidence: "No year metadata was supplied or inferred".to_string(),
                    candidate_item_id: None,
                }));
            }
            reasons.extend(
                duplicate_candidates
                    .iter()
                    .map(duplicate_candidate_review_reason)
                    .map(|reason| review_reason_line(&reason)),
            );
            reasons.extend(additional_review_reasons.iter().map(review_reason_line));
            reasons
        },
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
    let mut review_reasons = vec![
        review_reason_line(&ReviewReason {
            id: "unknown-creator".to_string(),
            kind: "unknown-metadata".to_string(),
            target_field: Some("creator".to_string()),
            message: "Creator is unknown".to_string(),
            evidence: "Idea capture did not supply creator metadata".to_string(),
            candidate_item_id: None,
        }),
        review_reason_line(&ReviewReason {
            id: "unknown-year".to_string(),
            kind: "unknown-metadata".to_string(),
            target_field: Some("year".to_string()),
            message: "Year is unknown".to_string(),
            evidence: "Idea capture did not supply year metadata".to_string(),
            candidate_item_id: None,
        }),
    ];
    if capture.capture_method == "manual-fallback" {
        review_reasons.push(review_reason_line(&ReviewReason {
            id: "manual-fallback".to_string(),
            kind: "manual-fallback".to_string(),
            target_field: None,
            message: "Manual Fallback content needs review".to_string(),
            evidence: "Source extraction was unavailable; content was supplied manually"
                .to_string(),
            candidate_item_id: None,
        }));
    }
    review_reasons.extend(
        duplicate_candidates
            .iter()
            .map(duplicate_candidate_review_reason)
            .map(|reason| review_reason_line(&reason)),
    );
    let review_reasons = serde_yaml::to_string(&review_reasons).unwrap_or_else(|_| "[]\n".into());
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
review_reasons:\n\
{review_reasons}\
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
        review_reasons = review_reasons,
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

fn scan_item_record_entries(entries: Vec<ItemRecordEntry>) -> ItemRecordScan {
    let mut valid_records = Vec::new();
    let mut vault_problems = Vec::new();
    for entry in entries {
        match read_valid_item_record(entry) {
            Ok(record) => valid_records.push(record),
            Err(problem) => vault_problems.push(problem),
        }
    }
    ItemRecordScan {
        valid_records,
        vault_problems,
    }
}

fn read_valid_item_record(entry: ItemRecordEntry) -> Result<ValidItemRecord, VaultProblem> {
    let text = fs::read_to_string(&entry.record_path).map_err(|error| VaultProblem {
        path: entry.record_path.clone(),
        error: format!("Item Record could not be read: {error}"),
    })?;
    let frontmatter = text
        .strip_prefix("---\n")
        .and_then(|record| record.split_once("\n---\n").map(|parts| parts.0))
        .ok_or_else(|| VaultProblem {
            path: entry.record_path.clone(),
            error: "Item Record frontmatter delimiters are missing".to_string(),
        })?;
    let mapping =
        serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter).map_err(|error| VaultProblem {
            path: entry.record_path.clone(),
            error: format!("Item Record YAML parse error: {error}"),
        })?;

    let required = |key: &str| {
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .cloned()
            .and_then(yaml_scalar_string)
            .ok_or_else(|| VaultProblem {
                path: entry.record_path.clone(),
                error: format!("Item Record is missing required field `{key}`"),
            })
    };
    for key in ["id", "home_subvault", "item_type", "title"] {
        required(key)?;
    }
    match required("item_type")?.as_str() {
        "artwork" => {
            for key in ["creator", "year", "primary_file"] {
                required(key)?;
            }
        }
        "idea" => {
            for key in ["creator", "year", "primary_file", "source_link"] {
                required(key)?;
            }
        }
        item_type => {
            return Err(VaultProblem {
                path: entry.record_path.clone(),
                error: format!("Item Record has unsupported item type `{item_type}`"),
            });
        }
    }

    Ok(ValidItemRecord { entry, text })
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

fn review_reasons(record: &str) -> Vec<ReviewReason> {
    frontmatter_list(record, "review_reasons")
        .into_iter()
        .enumerate()
        .map(|(index, line)| parse_review_reason(&line, index))
        .collect()
}

fn parse_review_reason(line: &str, index: usize) -> ReviewReason {
    let parts = line.splitn(6, " | ").map(str::trim).collect::<Vec<_>>();
    if parts.len() >= 5 {
        let candidate_item_id = parts
            .get(5)
            .filter(|value| !value.is_empty())
            .map(|value| (*value).to_string())
            .or_else(|| {
                (parts[1] == "duplicate-candidate")
                    .then(|| {
                        parts[0]
                            .strip_prefix("duplicate-candidate-")
                            .map(str::to_string)
                    })
                    .flatten()
            });
        return ReviewReason {
            id: parts[0].to_string(),
            kind: parts[1].to_string(),
            target_field: (!parts[2].is_empty()).then(|| parts[2].to_string()),
            message: parts[3].to_string(),
            evidence: parts[4].to_string(),
            candidate_item_id,
        };
    }

    let kind = parts.first().copied().unwrap_or("review").to_string();
    let evidence = parts.iter().skip(1).copied().collect::<Vec<_>>().join(": ");
    ReviewReason {
        id: format!("legacy-{kind}-{index}"),
        message: review_reason_message(&kind).to_string(),
        kind,
        target_field: None,
        evidence,
        candidate_item_id: None,
    }
}

fn review_reason_line(reason: &ReviewReason) -> String {
    let clean = |value: &str| value.replace('|', "/").replace(['\n', '\r'], " ");
    let line = format!(
        "{} | {} | {} | {} | {}",
        clean(&reason.id),
        clean(&reason.kind),
        clean(reason.target_field.as_deref().unwrap_or("")),
        clean(&reason.message),
        clean(&reason.evidence),
    );
    match reason.candidate_item_id.as_deref() {
        Some(candidate_item_id) => format!("{line} | {}", clean(candidate_item_id)),
        None => line,
    }
}

fn review_reason_sequence(reasons: &[ReviewReason]) -> serde_yaml::Value {
    serde_yaml::Value::Sequence(
        reasons
            .iter()
            .map(review_reason_line)
            .map(serde_yaml::Value::String)
            .collect(),
    )
}

fn duplicate_candidate_review_reason(candidate: &DuplicateCandidate) -> ReviewReason {
    ReviewReason {
        id: format!("duplicate-candidate-{}", candidate.item_id),
        kind: "duplicate-candidate".to_string(),
        target_field: None,
        message: "Possible overlap with another Saved Item".to_string(),
        evidence: format!("{}: {}", candidate.item_id, candidate.signal),
        candidate_item_id: Some(candidate.item_id.clone()),
    }
}

fn metadata_conflict_review_reason(field: &str, supplied: &str, inferred: &str) -> ReviewReason {
    ReviewReason {
        id: format!("conflicting-{field}"),
        kind: "conflicting-metadata".to_string(),
        target_field: Some(field.to_string()),
        message: format!("Conflicting {field} metadata"),
        evidence: format!("Import value {supplied}; filename suggested {inferred}"),
        candidate_item_id: None,
    }
}

fn review_reason_message(kind: &str) -> &str {
    match kind {
        "duplicate-candidate" => "Possible overlap with another Saved Item",
        "thumbnail-preview-unavailable" => "Thumbnail Preview is unavailable",
        "metadata-suggestion" => "Metadata Suggestion needs a decision",
        "manual-fallback" => "Manual Fallback content needs review",
        _ => "Saved Item needs review",
    }
}

fn derived_review_status(reasons: &[ReviewReason]) -> &'static str {
    if reasons.is_empty() {
        "reviewed"
    } else {
        "needs-review"
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
        .filter_map(|line| parse_item_link_line(&line))
        .collect()
}

fn parse_item_link_line(line: &str) -> Option<ItemLink> {
    let mut parts = line.splitn(3, " | ").map(str::trim);
    Some(ItemLink {
        link_type: parts.next()?.to_string(),
        label: parts.next()?.to_string(),
        target: parts.next()?.to_string(),
        target_in_vault_trash: false,
    })
}

fn rollback_text_mutations(applied: &[(PathBuf, String)]) -> Result<(), VaultError> {
    for (path, original) in applied.iter().rev() {
        atomic_write(path, original.as_bytes())
            .map_err(|_| VaultError::ReferenceCleanupRollbackFailed(path.clone()))?;
    }
    Ok(())
}

fn restore_file_snapshot(path: &Path, contents: Option<&[u8]>) -> Result<(), VaultError> {
    match contents {
        Some(contents) => atomic_write(path, contents)
            .map_err(|_| VaultError::ReferenceCleanupRollbackFailed(path.to_path_buf())),
        None if path.exists() => fs::remove_file(path)
            .map_err(|_| VaultError::ReferenceCleanupRollbackFailed(path.to_path_buf())),
        None => Ok(()),
    }
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
