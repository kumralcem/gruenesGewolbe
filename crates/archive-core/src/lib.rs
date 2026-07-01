use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

    pub fn validate(root: impl AsRef<Path>) -> Result<(), VaultError> {
        validate_vault_root(root.as_ref())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn add_artwork_item(&self, item: AddArtworkItem) -> Result<SavedItem, VaultError> {
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
        );
        fs::write(item_folder.join("record.md"), record)?;

        Ok(SavedItem {
            id,
            home_subvault: item.home_subvault,
            item_folder,
        })
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

            let inferred = infer_artwork_metadata(&path);
            imported.push(self.add_artwork_item(AddArtworkItem {
                source_file: path,
                home_subvault: "Paintings".to_string(),
                creator: inferred.creator,
                year: inferred.year,
                title: inferred.title,
                saving_reason: None,
            })?);
        }

        self.append_activity_log(&format!(
            "import-paintings\t{}\t{}",
            source_folder.display(),
            imported.len()
        ))?;

        Ok(imported)
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
            let thumbnail_file = self.cached_thumbnail_for(&saved_item, &primary_file)?;

            items.push(ArtworkGridItem {
                saved_item,
                title: required_frontmatter_value(&record.record_path, &text, "title")?,
                creator: required_frontmatter_value(&record.record_path, &text, "creator")?,
                year: required_frontmatter_value(&record.record_path, &text, "year")?,
                primary_file,
                thumbnail_file,
                review_status: required_frontmatter_value(
                    &record.record_path,
                    &text,
                    "review_status",
                )?,
            });
        }

        items.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.saved_item.id.cmp(&right.saved_item.id))
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

            let mut updated = text;
            if let Some(title) = update.title.as_deref() {
                updated = replace_frontmatter_value(&updated, "title", title);
            }
            if let Some(creator) = update.creator.as_deref() {
                updated = replace_frontmatter_value(&updated, "creator", creator);
            }
            if let Some(year) = update.year.as_deref() {
                updated = replace_frontmatter_value(&updated, "year", &format!("\"{year}\""));
            }
            if let Some(review_status) = update.review_status.as_deref() {
                updated = replace_frontmatter_value(&updated, "review_status", review_status);
            }
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

            let updated =
                replace_or_insert_frontmatter_value(&text, "tags", &existing_tags.join(", "));
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
            let updated =
                replace_or_insert_frontmatter_value(&text, "collections", &collections.join(", "));
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
            let text = fs::read_to_string(&record.record_path)?;
            let id = required_frontmatter_value(&record.record_path, &text, "id")?;

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
    ) -> Result<PathBuf, VaultError> {
        let thumbnails_dir = self.root.join(HIDDEN_STATE_DIR).join(THUMBNAILS_DIR);
        fs::create_dir_all(&thumbnails_dir)?;
        let thumbnail_file = thumbnails_dir.join(format!("{}.thumb", saved_item.id()));
        if !thumbnail_file.is_file() {
            fs::copy(primary_file, &thumbnail_file)?;
        }

        Ok(thumbnail_file)
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
                updated = replace_or_insert_frontmatter_value(&updated, "tags", &tags.join(", "));
            }
            if let Some(summary) = accepted_summary.as_deref() {
                updated = replace_or_append_markdown_section(&updated, "Summary", summary);
            }
            if !accepted_metadata.is_empty() {
                for suggestion in &accepted_metadata {
                    updated = replace_frontmatter_value(
                        &updated,
                        suggestion.field(),
                        &frontmatter_metadata_value(
                            suggestion.field(),
                            suggestion.suggested_value(),
                        ),
                    );
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
                updated = replace_frontmatter_value(&updated, "review_status", "needs-review");
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
                updated = replace_frontmatter_value(&updated, "review_status", "needs-review");
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
pub struct ArtworkGridItem {
    saved_item: SavedItem,
    title: String,
    creator: String,
    year: String,
    primary_file: PathBuf,
    thumbnail_file: PathBuf,
    review_status: String,
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
    MissingConfig(PathBuf),
    MissingSubvaults(PathBuf),
    MissingCollections(PathBuf),
    UnsupportedFormat(PathBuf),
    MissingSourceFile(PathBuf),
    MissingFileName(PathBuf),
    MissingImportFolder(PathBuf),
    SavedItemNotFound(String),
    CollectionNotFound(String),
    MalformedItemRecord(PathBuf),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingRoot(path) => write!(f, "vault root does not exist: {}", path.display()),
            Self::MissingConfig(path) => {
                write!(f, "vault config is missing: {}", path.display())
            }
            Self::MissingSubvaults(path) => {
                write!(f, "subvaults directory is missing: {}", path.display())
            }
            Self::MissingCollections(path) => {
                write!(f, "collections directory is missing: {}", path.display())
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
            Self::MissingImportFolder(path) => {
                write!(f, "import folder does not exist: {}", path.display())
            }
            Self::SavedItemNotFound(id) => write!(f, "saved item not found: {id}"),
            Self::CollectionNotFound(id) => write!(f, "collection not found: {id}"),
            Self::MalformedItemRecord(path) => {
                write!(f, "item record is malformed: {}", path.display())
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
    if !root.exists() {
        return Err(VaultError::MissingRoot(root.to_path_buf()));
    }

    let config_path = root.join(VAULT_CONFIG_FILE);
    if !config_path.is_file() {
        return Err(VaultError::MissingConfig(config_path));
    }

    let config = fs::read_to_string(&config_path)?;
    if !config
        .lines()
        .any(|line| line.trim() == "format_version = 1")
    {
        return Err(VaultError::UnsupportedFormat(config_path));
    }

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

fn default_config() -> String {
    format!("format_version = 1\nname = \"{DEFAULT_VAULT_NAME}\"\n")
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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("item-{nanos}")
}

fn imported_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
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

fn artwork_record(
    id: &str,
    item: &AddArtworkItem,
    primary_file: &str,
    import_original_filename: &str,
    import_source_path: &Path,
    imported_at: &str,
    file_fingerprint: &str,
    duplicate_candidates: &[DuplicateCandidate],
) -> String {
    let creator = item.creator.as_deref().unwrap_or("Unknown Creator");
    let year = item.year.as_deref().unwrap_or("Unknown Year");
    let saving_reason = item.saving_reason.as_deref().unwrap_or("");
    let duplicate_candidates = duplicate_candidate_frontmatter(duplicate_candidates);
    let import_source_folder = import_source_path
        .parent()
        .map(Path::display)
        .map(|display| display.to_string())
        .unwrap_or_default();

    format!(
        "---\n\
id: {id}\n\
item_type: artwork\n\
home_subvault: {home_subvault}\n\
title: {title}\n\
creator: {creator}\n\
year: \"{year}\"\n\
primary_file: {primary_file}\n\
import_original_filename: {import_original_filename}\n\
import_source_path: {import_source_path}\n\
import_source_folder: {import_source_folder}\n\
imported_at: {imported_at}\n\
file_fingerprint: {file_fingerprint}\n\
duplicate_candidates: {duplicate_candidates}\n\
review_status: needs-review\n\
---\n\
\n\
# {title}\n\
\n\
## Saving Reason\n\
\n\
{saving_reason}\n",
        home_subvault = item.home_subvault,
        title = item.title,
        import_source_path = import_source_path.display(),
        import_source_folder = import_source_folder,
    )
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
    frontmatter_value(record, key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn frontmatter_value(record: &str, key: &str) -> Option<String> {
    let mut lines = record.lines();
    if lines.next()? != "---" {
        return None;
    }

    let prefix = format!("{key}: ");
    for line in lines {
        if line == "---" {
            return None;
        }

        if let Some(value) = line.strip_prefix(&prefix) {
            return Some(value.trim_matches('"').to_string());
        }
    }

    None
}

fn replace_or_insert_frontmatter_value(record: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}: ");
    let mut replaced = false;
    let mut inserted = false;
    let mut output = Vec::new();

    for line in record.lines() {
        if line.starts_with(&prefix) {
            output.push(format!("{prefix}{value}"));
            replaced = true;
            continue;
        }

        if !replaced && !inserted && line == "---" && !output.is_empty() {
            output.push(format!("{prefix}{value}"));
            inserted = true;
        }

        output.push(line.to_string());
    }

    output.join("\n") + "\n"
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

fn replace_frontmatter_value(record: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}: ");
    record
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                format!("{prefix}{value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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

fn frontmatter_metadata_value(field: &str, value: &str) -> String {
    if field == "year" {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
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
