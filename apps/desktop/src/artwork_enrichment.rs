#[cfg(feature = "tauri-runtime")]
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::AiBudgetMode;
#[cfg(feature = "tauri-runtime")]
use gruenes_gewolbe_core::{AiMetadataSuggestion, AiProviderResponse, BetterFileCandidate};
use serde::{Deserialize, Serialize};

const HIDDEN_STATE_DIR: &str = ".gruenesgewolbe";
const CHECKPOINT_FILE: &str = "artwork-enrichment-checkpoint.json";
pub const DEFAULT_MAX_ITEMS: usize = 25;
pub const DEFAULT_MAX_REQUESTS: usize = 25;
pub const DEFAULT_MAX_DURATION_SECONDS: u64 = 600;
pub const MAX_ITEMS_PER_RUN: usize = 250;
pub const MAX_REQUESTS_PER_RUN: usize = 250;
pub const MAX_DURATION_SECONDS: u64 = 3_600;
pub const MAX_THUMBNAIL_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkEnrichmentOptions {
    pub budget_mode: String,
    #[serde(default = "default_max_items")]
    pub max_items: usize,
    #[serde(default = "default_max_requests")]
    pub max_requests: usize,
    #[serde(default = "default_max_duration_seconds")]
    pub max_duration_seconds: u64,
    #[serde(default)]
    pub rerun_completed: bool,
}

fn default_max_items() -> usize {
    DEFAULT_MAX_ITEMS
}

fn default_max_requests() -> usize {
    DEFAULT_MAX_REQUESTS
}

fn default_max_duration_seconds() -> u64 {
    DEFAULT_MAX_DURATION_SECONDS
}

impl ArtworkEnrichmentOptions {
    pub fn validate(&self) -> Result<AiBudgetMode, String> {
        if !(1..=MAX_ITEMS_PER_RUN).contains(&self.max_items) {
            return Err(format!(
                "maxItems must be between 1 and {MAX_ITEMS_PER_RUN}"
            ));
        }
        if !(1..=MAX_REQUESTS_PER_RUN).contains(&self.max_requests) {
            return Err(format!(
                "maxRequests must be between 1 and {MAX_REQUESTS_PER_RUN}"
            ));
        }
        if !(10..=MAX_DURATION_SECONDS).contains(&self.max_duration_seconds) {
            return Err(format!(
                "maxDurationSeconds must be between 10 and {MAX_DURATION_SECONDS}"
            ));
        }
        match self.budget_mode.as_str() {
            "off" => Err("AI budget is off".to_string()),
            "cheap" => Ok(AiBudgetMode::Cheap),
            "standard" => Ok(AiBudgetMode::Standard),
            "deep" => Ok(AiBudgetMode::Deep),
            _ => Err(format!("unsupported AI budget mode: {}", self.budget_mode)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtworkEnrichmentStatus {
    Running,
    Paused,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkEnrichmentFailure {
    pub item_id: String,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkEnrichmentProgress {
    pub run_id: String,
    pub processed: usize,
    pub total: usize,
    pub enriched: usize,
    pub failed: usize,
    pub skipped: usize,
    pub request_count: usize,
    pub remaining: usize,
    pub status: ArtworkEnrichmentStatus,
    pub current_item_title: Option<String>,
    pub failures: Vec<ArtworkEnrichmentFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkCandidateSnapshot {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub source_link: Option<String>,
    pub record_revision: String,
    pub thumbnail_file: PathBuf,
    pub thumbnail_fingerprint: String,
}

impl ArtworkCandidateSnapshot {
    pub fn from_thumbnail(
        id: String,
        title: String,
        creator: String,
        year: String,
        source_link: Option<String>,
        record_revision: String,
        thumbnail_file: PathBuf,
    ) -> Result<Self, String> {
        let bytes = read_bounded_thumbnail(&thumbnail_file)?;
        Ok(Self {
            id,
            title,
            creator,
            year,
            source_link,
            record_revision,
            thumbnail_file,
            thumbnail_fingerprint: fingerprint(&bytes),
        })
    }

    pub fn planned(
        id: String,
        title: String,
        creator: String,
        year: String,
        source_link: Option<String>,
        record_revision: String,
    ) -> Self {
        Self {
            id,
            title,
            creator,
            year,
            source_link,
            record_revision,
            thumbnail_file: PathBuf::new(),
            thumbnail_fingerprint: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointItemStatus {
    Pending,
    Enriched,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointItem {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub year: String,
    pub source_link: Option<String>,
    pub record_revision: String,
    pub thumbnail_file: PathBuf,
    pub thumbnail_fingerprint: String,
    #[serde(default)]
    pub skip_if_unchanged: bool,
    pub status: CheckpointItemStatus,
    pub error: Option<String>,
}

impl From<ArtworkCandidateSnapshot> for CheckpointItem {
    fn from(snapshot: ArtworkCandidateSnapshot) -> Self {
        Self {
            id: snapshot.id,
            title: snapshot.title,
            creator: snapshot.creator,
            year: snapshot.year,
            source_link: snapshot.source_link,
            record_revision: snapshot.record_revision,
            thumbnail_file: snapshot.thumbnail_file,
            thumbnail_fingerprint: snapshot.thumbnail_fingerprint,
            skip_if_unchanged: false,
            status: CheckpointItemStatus::Pending,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkEnrichmentCheckpoint {
    pub version: u32,
    pub run_id: String,
    pub vault_root: PathBuf,
    pub budget_mode: String,
    pub max_items: usize,
    pub max_requests: usize,
    pub max_duration_seconds: u64,
    pub request_count: usize,
    pub status: ArtworkEnrichmentStatus,
    pub items: Vec<CheckpointItem>,
}

impl ArtworkEnrichmentCheckpoint {
    pub fn start(
        vault_root: &Path,
        options: &ArtworkEnrichmentOptions,
        candidates: Vec<ArtworkCandidateSnapshot>,
    ) -> Result<Self, String> {
        options.validate()?;
        let previous = load_checkpoint(vault_root).ok();
        let mut items = candidates
            .into_iter()
            .map(CheckpointItem::from)
            .collect::<Vec<_>>();

        if !options.rerun_completed {
            if let Some(previous) = previous {
                for item in &mut items {
                    if let Some(old) = previous.items.iter().find(|old| {
                        old.id == item.id
                            && matches!(
                                old.status,
                                CheckpointItemStatus::Enriched | CheckpointItemStatus::Skipped
                            )
                            && old.record_revision == item.record_revision
                    }) {
                        item.thumbnail_fingerprint = old.thumbnail_fingerprint.clone();
                        item.skip_if_unchanged = !old.thumbnail_fingerprint.is_empty();
                    }
                }
            }
        }

        Ok(Self {
            version: 1,
            run_id: new_run_id(),
            vault_root: vault_root.to_path_buf(),
            budget_mode: options.budget_mode.clone(),
            max_items: options.max_items,
            max_requests: options.max_requests,
            max_duration_seconds: options.max_duration_seconds,
            request_count: 0,
            status: ArtworkEnrichmentStatus::Running,
            items,
        })
    }

    pub fn resume(vault_root: &Path, run_id: &str) -> Result<Self, String> {
        let mut checkpoint = load_checkpoint(vault_root)?;
        if checkpoint.run_id != run_id {
            return Err(format!("enrichment run was not found: {run_id}"));
        }
        if checkpoint.vault_root != vault_root {
            return Err("enrichment checkpoint belongs to a different Vault".to_string());
        }
        if checkpoint.status == ArtworkEnrichmentStatus::Completed {
            return Ok(checkpoint);
        }
        checkpoint.status = ArtworkEnrichmentStatus::Running;
        Ok(checkpoint)
    }

    pub fn next_pending_index(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.status == CheckpointItemStatus::Pending)
    }

    pub fn item(&self, index: usize) -> &CheckpointItem {
        &self.items[index]
    }

    pub fn mark_enriched(&mut self, index: usize, new_revision: String) {
        let item = &mut self.items[index];
        item.record_revision = new_revision;
        item.status = CheckpointItemStatus::Enriched;
        item.error = None;
    }

    pub fn set_thumbnail_snapshot(
        &mut self,
        index: usize,
        thumbnail_file: PathBuf,
        thumbnail_fingerprint: String,
    ) {
        self.items[index].thumbnail_file = thumbnail_file;
        self.items[index].thumbnail_fingerprint = thumbnail_fingerprint;
        self.items[index].skip_if_unchanged = false;
    }

    pub fn should_skip_unchanged(&self, index: usize, current_fingerprint: &str) -> bool {
        self.items[index].skip_if_unchanged
            && self.items[index].thumbnail_fingerprint == current_fingerprint
    }

    pub fn mark_skipped(&mut self, index: usize) {
        self.items[index].status = CheckpointItemStatus::Skipped;
        self.items[index].error = None;
    }

    pub fn mark_failed(&mut self, index: usize, reason: String) {
        let item = &mut self.items[index];
        item.status = CheckpointItemStatus::Failed;
        item.error = Some(reason);
    }

    pub fn increment_requests(&mut self) {
        self.request_count += 1;
    }

    pub fn finish_or_pause(&mut self) {
        self.status = if self.next_pending_index().is_some() {
            ArtworkEnrichmentStatus::Paused
        } else {
            ArtworkEnrichmentStatus::Completed
        };
    }

    pub fn cancel(&mut self) {
        self.status = ArtworkEnrichmentStatus::Cancelled;
    }

    pub fn progress(&self, current_item_title: Option<String>) -> ArtworkEnrichmentProgress {
        let enriched = self
            .items
            .iter()
            .filter(|item| item.status == CheckpointItemStatus::Enriched)
            .count();
        let failed = self
            .items
            .iter()
            .filter(|item| item.status == CheckpointItemStatus::Failed)
            .count();
        let skipped = self
            .items
            .iter()
            .filter(|item| item.status == CheckpointItemStatus::Skipped)
            .count();
        let remaining = self
            .items
            .iter()
            .filter(|item| item.status == CheckpointItemStatus::Pending)
            .count();
        ArtworkEnrichmentProgress {
            run_id: self.run_id.clone(),
            processed: enriched + failed + skipped,
            total: self.items.len(),
            enriched,
            failed,
            skipped,
            request_count: self.request_count,
            remaining,
            status: self.status,
            current_item_title,
            failures: self
                .items
                .iter()
                .filter_map(|item| {
                    item.error.as_ref().map(|reason| ArtworkEnrichmentFailure {
                        item_id: item.id.clone(),
                        title: item.title.clone(),
                        reason: reason.clone(),
                    })
                })
                .collect(),
        }
    }
}

pub(crate) fn checkpoint_path(vault_root: &Path) -> PathBuf {
    vault_root.join(HIDDEN_STATE_DIR).join(CHECKPOINT_FILE)
}

pub fn save_checkpoint(checkpoint: &ArtworkEnrichmentCheckpoint) -> Result<(), String> {
    let path = checkpoint_path(&checkpoint.vault_root);
    let parent = path
        .parent()
        .ok_or_else(|| "checkpoint path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!("enrichment checkpoint directory could not be created: {error}")
    })?;
    let temporary = parent.join(format!(".{CHECKPOINT_FILE}.tmp"));
    let bytes = serde_json::to_vec_pretty(checkpoint)
        .map_err(|error| format!("enrichment checkpoint could not be encoded: {error}"))?;
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("enrichment checkpoint could not be written: {error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("enrichment checkpoint could not be written: {error}"))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("enrichment checkpoint could not be saved: {error}"))
}

pub(crate) fn load_checkpoint(vault_root: &Path) -> Result<ArtworkEnrichmentCheckpoint, String> {
    let path = checkpoint_path(vault_root);
    let checkpoint: ArtworkEnrichmentCheckpoint = serde_json::from_slice(
        &fs::read(&path)
            .map_err(|error| format!("enrichment checkpoint could not be read: {error}"))?,
    )
    .map_err(|error| format!("enrichment checkpoint is invalid: {error}"))?;
    if checkpoint.version != 1 {
        return Err(format!(
            "unsupported enrichment checkpoint version: {}",
            checkpoint.version
        ));
    }
    Ok(checkpoint)
}

pub fn latest_progress(vault_root: &Path) -> Result<ArtworkEnrichmentProgress, String> {
    load_checkpoint(vault_root).map(|checkpoint| checkpoint.progress(None))
}

pub fn latest_progress_if_exists(
    vault_root: &Path,
) -> Result<Option<ArtworkEnrichmentProgress>, String> {
    if !checkpoint_path(vault_root).is_file() {
        return Ok(None);
    }
    latest_progress(vault_root).map(Some)
}

pub fn read_bounded_thumbnail(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Thumbnail Preview could not be read: {error}"))?;
    if !metadata.is_file() {
        return Err("Thumbnail Preview is not a file".to_string());
    }
    if metadata.len() > MAX_THUMBNAIL_BYTES as u64 {
        return Err(format!(
            "Thumbnail Preview exceeds the {} MiB enrichment limit",
            MAX_THUMBNAIL_BYTES / (1024 * 1024)
        ));
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| {
            file.take(MAX_THUMBNAIL_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|error| format!("Thumbnail Preview could not be read: {error}"))?;
    if bytes.len() > MAX_THUMBNAIL_BYTES {
        return Err(format!(
            "Thumbnail Preview exceeds the {} MiB enrichment limit",
            MAX_THUMBNAIL_BYTES / (1024 * 1024)
        ));
    }
    Ok(bytes)
}

pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}:{}", bytes.len())
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("artwork-{nanos}-{}", std::process::id())
}

#[cfg(feature = "tauri-runtime")]
pub struct ArtworkResponsesProvider {
    client: reqwest::blocking::Client,
    api_key: String,
    model: String,
}

#[cfg(feature = "tauri-runtime")]
impl ArtworkResponsesProvider {
    pub fn new(api_key: String, model: String) -> Result<Self, String> {
        if api_key.trim().is_empty() || model.trim().is_empty() {
            return Err("OpenAI provider configuration is incomplete".to_string());
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|error| format!("OpenAI client could not start: {error}"))?;
        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    pub fn enrich_thumbnail(
        &self,
        item: &CheckpointItem,
        thumbnail_bytes: &[u8],
        budget_mode: AiBudgetMode,
        timeout: std::time::Duration,
    ) -> Result<AiProviderResponse, String> {
        if thumbnail_bytes.len() > MAX_THUMBNAIL_BYTES {
            return Err("Thumbnail Preview is larger than the request limit".to_string());
        }
        let timeout = timeout.min(std::time::Duration::from_secs(90));
        let (max_output_tokens, max_tool_calls) = match budget_mode {
            AiBudgetMode::Off => return Err("AI budget is off".to_string()),
            AiBudgetMode::Cheap => (500, 1),
            AiBudgetMode::Standard => (900, 2),
            AiBudgetMode::Deep => (1_400, 4),
        };
        let image_url = format!("data:image/png;base64,{}", base64_encode(thumbnail_bytes));
        let context = format!(
            "Existing Item Record fields (unknown values may be literal 'Unknown'): title={:?}, creator={:?}, year={:?}, saved source link={:?}. Analyze the image, then use web search when it can support factual identification. Treat the saved source link as a research lead, not proof by itself.",
            item.title, item.creator, item.year, item.source_link
        );
        let response = self
            .client
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(&self.api_key)
            .timeout(timeout)
            .json(&serde_json::json!({
                "model": self.model,
                "store": false,
                "max_output_tokens": max_output_tokens,
                "max_tool_calls": max_tool_calls,
                "include": ["web_search_call.action.sources"],
                "tools": [{"type": "web_search"}],
                "instructions": "You enrich a personal artwork archive. Describe only what is visible for descriptive tags. Use the web_search tool for factual identification. Never assert title, creator, or year from visual resemblance alone. Return factual identity suggestions only when supported by one or more source URLs found by web search. Prefer museum, collection, artist-estate, and other primary sources. Put uncertain or stylistic inferences in suggestions with calibrated confidence below 0.9. Do not propose moving, renaming, or replacing files. Tags must be short, descriptive qualities visible in the image (subject, mood, palette, composition), not unsupported factual identity claims.",
                "input": [{
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": context},
                        {"type": "input_image", "image_url": image_url, "detail": "low"}
                    ]
                }],
                "text": {"format": {
                    "type": "json_schema",
                    "name": "artwork_enrichment",
                    "strict": true,
                    "schema": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "tags": {"type": "array", "maxItems": 12, "items": {"type": "string"}},
                            "suggestions": {"type": "array", "maxItems": 8, "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "properties": {
                                    "field": {"type": "string", "enum": ["title", "creator", "year", "style", "medium", "subjects", "mood"]},
                                    "value": {"type": "string"},
                                    "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                                    "source_urls": {"type": "array", "items": {"type": "string"}}
                                },
                                "required": ["field", "value", "confidence", "source_urls"]
                            }}
                        },
                        "required": ["tags", "suggestions"]
                    }
                }}
            }))
            .send()
            .map_err(|error| format!("OpenAI artwork request failed: {error}"))?;
        use std::io::Read;
        if !response.status().is_success() {
            let status = response.status();
            let mut error_bytes = Vec::new();
            response
                .take(MAX_RESPONSE_BYTES + 1)
                .read_to_end(&mut error_bytes)
                .map_err(|error| format!("OpenAI error response could not be read: {error}"))?;
            let detail = serde_json::from_slice::<serde_json::Value>(&error_bytes)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(|error| error.get("message"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "the provider did not accept this request".to_string());
            return Err(format!(
                "OpenAI provider rejected the request (HTTP {}): {detail}",
                status.as_u16()
            ));
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("OpenAI artwork response could not be read: {error}"))?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err("OpenAI artwork response exceeded the 2 MiB limit".to_string());
        }
        parse_openai_artwork_response(&bytes)
    }
}

#[cfg(feature = "tauri-runtime")]
#[derive(Debug, Deserialize)]
struct ProviderPayload {
    tags: Vec<String>,
    suggestions: Vec<ProviderSuggestion>,
}

#[cfg(feature = "tauri-runtime")]
#[derive(Debug, Deserialize)]
struct ProviderSuggestion {
    field: String,
    value: String,
    confidence: f32,
    source_urls: Vec<String>,
}

#[cfg(feature = "tauri-runtime")]
fn parse_openai_artwork_response(bytes: &[u8]) -> Result<AiProviderResponse, String> {
    let response: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("OpenAI artwork response was invalid: {error}"))?;
    if response.get("status").and_then(serde_json::Value::as_str) != Some("completed") {
        let detail = response
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("response did not complete");
        return Err(format!(
            "OpenAI artwork response did not complete: {detail}"
        ));
    }
    let researched_sources = web_search_sources(&response);
    let output_text = response
        .get("output")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("content").and_then(serde_json::Value::as_array))
        .flatten()
        .find_map(|content| {
            (content.get("type").and_then(serde_json::Value::as_str) == Some("output_text"))
                .then(|| content.get("text").and_then(serde_json::Value::as_str))
                .flatten()
        })
        .ok_or_else(|| "OpenAI returned no artwork enrichment output".to_string())?;
    let payload: ProviderPayload = serde_json::from_str(output_text)
        .map_err(|error| format!("OpenAI artwork output did not match its schema: {error}"))?;
    let factual_fields = ["title", "creator", "year"];
    let suggestions = payload
        .suggestions
        .into_iter()
        .filter_map(|suggestion| {
            let mut sources = suggestion
                .source_urls
                .into_iter()
                .filter(|url| researched_sources.contains(url))
                .collect::<Vec<_>>();
            sources.sort();
            sources.dedup();
            if factual_fields.contains(&suggestion.field.as_str()) && sources.is_empty() {
                return None;
            }
            let provenance = if sources.is_empty() {
                "OpenAI vision analysis of bounded Thumbnail Preview".to_string()
            } else {
                format!("OpenAI web research: {}", sources.join(", "))
            };
            Some(AiMetadataSuggestion {
                field: suggestion.field,
                suggested_value: suggestion.value,
                confidence: suggestion.confidence.clamp(0.0, 1.0),
                provenance,
            })
        })
        .collect();
    Ok(AiProviderResponse {
        summary: None,
        tags: payload.tags,
        suggestions,
        better_file_candidates: Vec::<BetterFileCandidate>::new(),
        estimated_cost_cents: 0,
    })
}

#[cfg(feature = "tauri-runtime")]
fn web_search_sources(response: &serde_json::Value) -> HashSet<String> {
    let mut sources = HashSet::new();
    for output in response
        .get("output")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if output.get("type").and_then(serde_json::Value::as_str) == Some("web_search_call") {
            if let Some(values) = output
                .get("action")
                .and_then(|action| action.get("sources"))
                .and_then(serde_json::Value::as_array)
            {
                for value in values {
                    if let Some(url) = value.get("url").and_then(serde_json::Value::as_str) {
                        if valid_source_url(url) {
                            sources.insert(url.to_string());
                        }
                    }
                }
            }
        }
    }
    sources
}

#[cfg(feature = "tauri-runtime")]
fn valid_source_url(value: &str) -> bool {
    value.len() <= 2_048
        && reqwest::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        encoded.push(TABLE[(a >> 2) as usize] as char);
        encoded.push(TABLE[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(c & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("gg-enrichment-{name}-{unique}"))
    }

    fn options() -> ArtworkEnrichmentOptions {
        ArtworkEnrichmentOptions {
            budget_mode: "standard".to_string(),
            max_items: 25,
            max_requests: 25,
            max_duration_seconds: 600,
            rerun_completed: false,
        }
    }

    fn candidate(root: &Path, id: &str, revision: &str) -> ArtworkCandidateSnapshot {
        let thumbnail_file = root.join(format!("{id}.png"));
        fs::create_dir_all(root).unwrap();
        fs::write(&thumbnail_file, format!("thumbnail-{id}")).unwrap();
        ArtworkCandidateSnapshot::from_thumbnail(
            id.to_string(),
            format!("Title {id}"),
            "Unknown".to_string(),
            "Unknown".to_string(),
            None,
            revision.to_string(),
            thumbnail_file,
        )
        .unwrap()
    }

    #[test]
    fn checkpoint_round_trip_resumes_pending_items() {
        let root = temp_root("resume");
        let mut checkpoint = ArtworkEnrichmentCheckpoint::start(
            &root,
            &options(),
            vec![
                candidate(&root, "one", "revision-1"),
                candidate(&root, "two", "revision-2"),
            ],
        )
        .unwrap();
        checkpoint.mark_enriched(0, "enriched-revision".to_string());
        checkpoint.finish_or_pause();
        save_checkpoint(&checkpoint).unwrap();

        let resumed = ArtworkEnrichmentCheckpoint::resume(&root, &checkpoint.run_id).unwrap();
        assert_eq!(resumed.next_pending_index(), Some(1));
        assert_eq!(resumed.progress(None).enriched, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_completed_items_are_skipped_but_changed_records_are_eligible() {
        let root = temp_root("skip");
        let unchanged = candidate(&root, "one", "revision-1");
        let mut first =
            ArtworkEnrichmentCheckpoint::start(&root, &options(), vec![unchanged.clone()]).unwrap();
        first.mark_enriched(0, "revision-1".to_string());
        first.finish_or_pause();
        save_checkpoint(&first).unwrap();

        let second = ArtworkEnrichmentCheckpoint::start(
            &root,
            &options(),
            vec![unchanged, candidate(&root, "two", "revision-new")],
        )
        .unwrap();
        assert_eq!(second.progress(None).skipped, 0);
        assert_eq!(second.next_pending_index(), Some(0));
        assert!(second.should_skip_unchanged(0, &second.items[0].thumbnail_fingerprint));
        assert!(!second.should_skip_unchanged(0, "a-changed-thumbnail"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_limit_does_not_drop_later_paintings_from_the_checkpoint() {
        let root = temp_root("all-items");
        let mut limited = options();
        limited.max_items = 1;
        let checkpoint = ArtworkEnrichmentCheckpoint::start(
            &root,
            &limited,
            vec![
                candidate(&root, "one", "revision-1"),
                candidate(&root, "two", "revision-2"),
            ],
        )
        .unwrap();
        assert_eq!(checkpoint.items.len(), 2);
        assert_eq!(checkpoint.progress(None).remaining, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn options_enforce_item_request_and_duration_bounds() {
        let mut value = options();
        value.max_items = MAX_ITEMS_PER_RUN + 1;
        assert!(value.validate().unwrap_err().contains("maxItems"));
        value = options();
        value.max_requests = 0;
        assert!(value.validate().unwrap_err().contains("maxRequests"));
        value = options();
        value.max_duration_seconds = MAX_DURATION_SECONDS + 1;
        assert!(value.validate().unwrap_err().contains("maxDurationSeconds"));
    }

    #[test]
    fn base64_encoder_handles_padding() {
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[cfg(feature = "tauri-runtime")]
    #[test]
    fn factual_suggestions_require_urls_from_actual_web_search_sources() {
        let response = serde_json::json!({
            "status": "completed",
            "output": [
                {"type": "web_search_call", "action": {"sources": [{"url": "https://museum.example/art/1"}]}},
                {"type": "message", "content": [{"type": "output_text", "text": serde_json::json!({
                    "tags": ["night scene"],
                    "suggestions": [
                        {"field": "creator", "value": "Invented", "confidence": 0.99, "source_urls": ["https://invented.example/nope"]},
                        {"field": "title", "value": "Supported", "confidence": 0.95, "source_urls": ["https://museum.example/art/1"]},
                        {"field": "mood", "value": "quiet", "confidence": 0.65, "source_urls": []}
                    ]
                }).to_string()}]}
            ]
        });
        let parsed = parse_openai_artwork_response(response.to_string().as_bytes()).unwrap();
        assert_eq!(parsed.suggestions.len(), 2);
        assert!(parsed
            .suggestions
            .iter()
            .all(|suggestion| suggestion.field != "creator"));
        assert!(parsed.suggestions[0]
            .provenance
            .contains("https://museum.example/art/1"));
    }
}
