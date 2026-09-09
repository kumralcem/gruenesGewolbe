use std::io::Read;
use std::time::Duration;

use gruenes_gewolbe_core::{SourceExtraction, SourceExtractionRequest, SourceExtractor};
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::Url;
use scraper::{Html, Selector};

const MAX_PAGE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;

pub struct BoundedSourceExtractor {
    client: Client,
    idea_client: Client,
}

impl BoundedSourceExtractor {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("GruenesGewoelbe/0.1 personal archive")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if should_follow_supported_redirect(attempt.previous(), attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| format!("source extractor could not start: {error}"))?;
        let idea_client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("GruenesGewoelbe/0.1 personal archive")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if should_follow_page_redirect(attempt.previous().len(), attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| format!("Idea Source extractor could not start: {error}"))?;
        Ok(Self {
            client,
            idea_client,
        })
    }

    pub fn extract_idea(&self, request: SourceExtractionRequest) -> SourceExtraction {
        let source = match Url::parse(&request.source_link) {
            Ok(source) if is_public_https_url(&source) => source,
            _ => return fallback("Use a public HTTPS source link for automatic capture"),
        };
        self.extract_page_text(&source)
            .unwrap_or_else(|reason| fallback(&reason))
    }

    fn extract_page_text(&self, source: &Url) -> Result<SourceExtraction, String> {
        let response = self
            .idea_client
            .get(source.clone())
            .send()
            .and_then(Response::error_for_status)
            .map_err(|error| format!("Source page could not be read: {error}"))?;
        if !is_public_https_url(response.url()) {
            return Err("Source page redirected outside public HTTPS".to_string());
        }
        let is_html = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value.starts_with("text/html") || value.starts_with("application/xhtml+xml")
            });
        if !is_html {
            return Err("Source did not return an HTML page".to_string());
        }
        let html = String::from_utf8(read_bounded(response, MAX_PAGE_BYTES, "Source page")?)
            .map_err(|_| "Source page was not UTF-8 text".to_string())?;
        let title = meta_content(&html, "og:title").or_else(|| html_title(&html));
        let cleaned_text = if is_x_host(source) {
            x_post_text(&html).ok_or_else(|| {
                "X did not expose public post text; paste the post text instead".to_string()
            })?
        } else {
            if looks_like_access_challenge(&html) {
                return Err("Source page returned an access or login challenge; paste the source text instead".to_string());
            }
            let document = Html::parse_document(&html);
            let has_main_content = ["article", "main"].iter().any(|selector| {
                Selector::parse(selector)
                    .ok()
                    .is_some_and(|selector| document.select(&selector).next().is_some())
            });
            let cleaned = cleaned_main_text(&html);
            if !has_main_content {
                let paragraph_count = Selector::parse("body p")
                    .ok()
                    .map_or(0, |selector| document.select(&selector).count());
                if cleaned.chars().count() < 200 || paragraph_count < 2 {
                    return Err("Source page did not expose a clear main article; paste the source text instead".to_string());
                }
            }
            cleaned
        };
        if cleaned_text.trim().is_empty() {
            return Err(
                "Source page did not expose readable text; paste the source text instead"
                    .to_string(),
            );
        }
        Ok(SourceExtraction::ExtractedText {
            title,
            cleaned_text,
        })
    }

    fn extract_wikimedia(&self, source: &Url) -> Result<SourceExtraction, String> {
        let (title, candidates) = if source.host_str() == Some("upload.wikimedia.org") {
            (None, upload_url_candidates(source))
        } else {
            let page = self
                .client
                .get(source.clone())
                .send()
                .and_then(Response::error_for_status)
                .map_err(|error| format!("Wikimedia page could not be read: {error}"))?;
            let html = read_bounded(page, MAX_PAGE_BYTES, "Wikimedia page")?;
            let html = String::from_utf8(html)
                .map_err(|_| "Wikimedia page was not UTF-8 text".to_string())?;
            let title = meta_content(&html, "og:title")
                .map(|value| value.trim_end_matches(" - Wikimedia Commons").to_string());
            let candidates = expand_wikimedia_candidates(&html, source);
            if candidates.is_empty() {
                return Err("Wikimedia page did not expose a preserved image".to_string());
            }
            (title, candidates)
        };
        self.download_first_in_budget(candidates, title, "Wikimedia")
    }

    fn extract_x(&self, source: &Url) -> Result<SourceExtraction, String> {
        let page = self
            .client
            .get(source.clone())
            .send()
            .and_then(Response::error_for_status)
            .map_err(|error| format!("X page could not be read: {error}"))?;
        let html = read_bounded(page, MAX_PAGE_BYTES, "X page")?;
        let html = String::from_utf8(html).map_err(|_| "X page was not UTF-8 text".to_string())?;
        let title = meta_content(&html, "og:title").or_else(|| x_title_from_url(source));
        if let Some(image_link) = x_post_media_image(&html) {
            let image_url =
                Url::parse(&image_link).map_err(|_| "X image link was invalid".to_string())?;
            if !is_x_media_host(&image_url) {
                return Err("X image link left the supported media host".to_string());
            }
            return self.download_image(image_url, title);
        }
        Err("X.com did not expose a public post image".to_string())
    }

    fn download_first_in_budget(
        &self,
        candidates: Vec<Url>,
        title: Option<String>,
        label: &str,
    ) -> Result<SourceExtraction, String> {
        let mut last_error = format!("{label} page did not expose a preserved image");
        for image_url in candidates {
            match self.download_image(image_url, title.clone()) {
                Ok(extracted) => return Ok(extracted),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    fn download_image(
        &self,
        image_url: Url,
        title: Option<String>,
    ) -> Result<SourceExtraction, String> {
        if !is_allowed_image_host(&image_url) {
            return Err("image link left the supported media host".to_string());
        }
        if let Some(length) = self.head_content_length(&image_url) {
            if length > MAX_IMAGE_BYTES {
                return Err(format!(
                    "source image exceeds the {} MiB capture limit",
                    MAX_IMAGE_BYTES / 1024 / 1024
                ));
            }
        }
        let response = self
            .client
            .get(image_url.clone())
            .send()
            .and_then(Response::error_for_status)
            .map_err(|error| format!("source image could not be downloaded: {error}"))?;
        let is_image = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("image/"));
        if !is_image {
            return Err("source response was not an image".to_string());
        }
        let file_name = image_file_name(&image_url);
        let title = title.or_else(|| {
            file_name
                .rsplit_once('.')
                .map(|(stem, _)| stem)
                .or(Some(file_name.as_str()))
                .map(|value| value.replace('_', " "))
        });
        let bytes = read_bounded(response, MAX_IMAGE_BYTES, "source image")?;
        Ok(SourceExtraction::ExtractedImage {
            title,
            file_name,
            bytes,
        })
    }

    fn head_content_length(&self, url: &Url) -> Option<u64> {
        self.client
            .head(url.clone())
            .send()
            .ok()
            .and_then(|response| response.content_length())
    }
}

impl SourceExtractor for BoundedSourceExtractor {
    fn extract(&self, request: SourceExtractionRequest) -> SourceExtraction {
        let source = match Url::parse(&request.source_link) {
            Ok(source) if source.scheme() == "https" && source.username().is_empty() => source,
            _ => return fallback("Use a public HTTPS source link for automatic capture"),
        };
        if is_x_host(&source) {
            return self
                .extract_x(&source)
                .unwrap_or_else(|reason| fallback(&reason));
        }
        if !is_wikimedia_host(&source) {
            return fallback("This source is not supported by bounded automatic extraction");
        }
        self.extract_wikimedia(&source)
            .unwrap_or_else(|reason| fallback(&reason))
    }
}

fn fallback(reason: &str) -> SourceExtraction {
    SourceExtraction::NeedsManualFallback {
        reason: reason.to_string(),
    }
}

fn is_wikimedia_host(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("commons.wikimedia.org" | "upload.wikimedia.org")
    )
}

fn is_x_host(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("x.com" | "www.x.com" | "twitter.com" | "www.twitter.com")
    )
}

fn is_x_media_host(url: &Url) -> bool {
    url.host_str() == Some("pbs.twimg.com")
}

fn is_allowed_image_host(url: &Url) -> bool {
    is_public_https_url(url)
        && (url.host_str() == Some("upload.wikimedia.org") || is_x_media_host(url))
}

fn should_follow_supported_redirect(previous: &[Url], destination: &Url) -> bool {
    if previous.len() >= 4 || !is_public_https_url(destination) {
        return false;
    }
    match previous.first() {
        Some(origin) if is_wikimedia_host(origin) => is_wikimedia_host(destination),
        Some(origin) if is_x_host(origin) => is_x_host(destination),
        Some(origin) if is_x_media_host(origin) => is_x_media_host(destination),
        _ => false,
    }
}

fn should_follow_page_redirect(previous: usize, url: &Url) -> bool {
    previous < 4 && is_public_https_url(url)
}

fn is_public_https_url(url: &Url) -> bool {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return false;
    }
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => is_public_ipv4(ip),
        Ok(std::net::IpAddr::V6(ip)) => {
            ip.to_ipv4_mapped().map(is_public_ipv4).unwrap_or_else(|| {
                !(ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local())
            })
        }
        Err(_) => true,
    }
}

fn is_public_ipv4(ip: std::net::Ipv4Addr) -> bool {
    !(ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified())
}

fn html_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let content_start = start + lower[start..].find('>')? + 1;
    let end = content_start + lower[content_start..].find("</title>")?;
    let title = decode_html_text(&html[content_start..end]);
    (!title.trim().is_empty()).then(|| title.trim().to_string())
}

fn cleaned_main_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let selectors = ["article", "main", "body"];
    let root = selectors.iter().find_map(|selector| {
        Selector::parse(selector)
            .ok()
            .and_then(|selector| document.select(&selector).next())
    });
    let Some(root) = root else {
        return String::new();
    };
    let block_selector =
        Selector::parse("h1,h2,h3,h4,h5,h6,p,li,blockquote,pre").expect("static selector");
    let blocks = root
        .select(&block_selector)
        .map(|element| {
            element
                .text()
                .flat_map(str::split_whitespace)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();
    if blocks.is_empty() {
        root.text()
            .flat_map(str::split_whitespace)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        blocks.join("\n\n")
    }
}

fn decode_html_text(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}

fn expand_wikimedia_candidates(html: &str, base: &Url) -> Vec<Url> {
    let mut urls: Vec<Url> = wikimedia_image_candidates(html)
        .into_iter()
        .filter_map(|link| parse_upload_url(&link, base))
        .collect();
    let original = urls
        .iter()
        .find(|url| !url.path().contains("/thumb/"))
        .cloned()
        .or_else(|| {
            urls.first().and_then(|url| {
                parse_upload_url(&unwrap_commons_thumb(url.as_str().to_string()), base)
            })
        });
    if let Some(original) = original {
        for extra in upload_url_candidates(&original) {
            if !urls.iter().any(|url| url.as_str() == extra.as_str()) {
                urls.push(extra);
            }
        }
    }
    urls
}

fn wikimedia_image_candidates(html: &str) -> Vec<String> {
    let mut original = None;
    let mut thumbs = Vec::new();
    collect_upload_links(
        section_after(html, "fullmedia").unwrap_or(""),
        &mut original,
        &mut thumbs,
    );
    collect_upload_links(
        section_after(html, "mw-filepage-resolutioninfo")
            .or_else(|| section_after(html, "filepage-other-resolutions"))
            .unwrap_or(""),
        &mut original,
        &mut thumbs,
    );
    if original.is_none() && thumbs.is_empty() {
        if let Some(image) = meta_content(html, "og:image") {
            collect_upload_link(&image, &mut original, &mut thumbs);
        }
    }
    if original.is_none() {
        if let Some((_, largest)) = thumbs.iter().max_by_key(|(width, _)| *width) {
            let unwrapped = unwrap_commons_thumb(largest.clone());
            if unwrapped != *largest {
                original = Some(unwrapped);
            }
        }
    }
    thumbs.sort_by(|left, right| right.0.cmp(&left.0));
    let mut candidates = Vec::new();
    if let Some(original) = original {
        candidates.push(original);
    }
    for (_, url) in thumbs {
        if !candidates.contains(&url) {
            candidates.push(url);
        }
    }
    candidates
}

fn collect_upload_links(
    section: &str,
    original: &mut Option<String>,
    thumbs: &mut Vec<(u32, String)>,
) {
    for tag in section.split('<') {
        let href = match attribute(tag, "href") {
            Some(href) if href.contains("upload.wikimedia.org") => decode_html_attribute(href),
            _ => continue,
        };
        collect_upload_link(&href, original, thumbs);
    }
}

fn collect_upload_link(href: &str, original: &mut Option<String>, thumbs: &mut Vec<(u32, String)>) {
    let href = normalize_upload_url(href);
    if !href.contains("upload.wikimedia.org") {
        return;
    }
    if href.contains("/thumb/") {
        let width = thumb_width(&href).unwrap_or(0);
        if !thumbs.iter().any(|(_, url)| url == &href) {
            thumbs.push((width, href));
        }
    } else if original.is_none() {
        *original = Some(href);
    }
}

fn normalize_upload_url(href: &str) -> String {
    let href = href.split_once('?').map(|(path, _)| path).unwrap_or(href);
    if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    }
}

fn parse_upload_url(link: &str, base: &Url) -> Option<Url> {
    let link = normalize_upload_url(link);
    let url = if link.starts_with("https://") {
        Url::parse(&link).ok()
    } else {
        base.join(&link).ok()
    }?;
    (url.scheme() == "https" && url.host_str() == Some("upload.wikimedia.org")).then_some(url)
}

fn upload_url_candidates(original: &Url) -> Vec<Url> {
    let mut candidates = vec![original.clone()];
    if original.path().contains("/thumb/") {
        if let Some(unwrapped) = parse_upload_url(
            &unwrap_commons_thumb(original.as_str().to_string()),
            original,
        ) {
            if unwrapped.as_str() != original.as_str() {
                candidates.insert(0, unwrapped.clone());
                candidates.extend(wikimedia_preview_urls(&unwrapped));
            }
        }
        return candidates;
    }
    candidates.extend(wikimedia_preview_urls(original));
    candidates
}

fn wikimedia_preview_urls(original: &Url) -> Vec<Url> {
    const WIDTHS: [u32; 8] = [3840, 2560, 1920, 1280, 960, 800, 640, 500];
    WIDTHS
        .into_iter()
        .filter_map(|width| wikimedia_thumb_url(original, width))
        .collect()
}

fn wikimedia_thumb_url(original: &Url, width: u32) -> Option<Url> {
    if original.host_str() != Some("upload.wikimedia.org") || original.path().contains("/thumb/") {
        return None;
    }
    let path = original.path().trim_start_matches('/');
    let mut parts: Vec<&str> = path.split('/').collect();
    let file_name = parts.last().copied().filter(|name| !name.is_empty())?;
    if parts.len() < 4 {
        return None;
    }
    parts.insert(2, "thumb");
    Url::parse(&format!(
        "https://upload.wikimedia.org/{}/{}px-{file_name}",
        parts.join("/"),
        width
    ))
    .ok()
}

fn thumb_width(url: &str) -> Option<u32> {
    let file_name = url.rsplit('/').next()?;
    let (width, rest) = file_name.split_once("px-")?;
    if rest.is_empty() {
        return None;
    }
    width.parse().ok()
}

fn section_after<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find(marker)?;
    Some(&html[start..(start + 64_000).min(html.len())])
}

#[cfg(test)]
fn first_in_budget(
    candidates: &[String],
    limit: u64,
    size_of: impl Fn(&str) -> Option<u64>,
) -> Option<String> {
    candidates
        .iter()
        .find(|url| size_of(url).is_none_or(|length| length <= limit))
        .cloned()
}

fn image_file_name(image_url: &Url) -> String {
    let mut name = image_url
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|name| !name.is_empty())
        .unwrap_or("captured-image")
        .to_string();
    if let Some((prefix, rest)) = name.split_once("px-") {
        if !prefix.is_empty() && prefix.chars().all(|ch| ch.is_ascii_digit()) && rest.contains('.')
        {
            name = rest.to_string();
        }
    }
    if let Some((stem, suffix)) = name.rsplit_once(':') {
        if matches!(suffix, "large" | "orig" | "small" | "medium" | "thumb") {
            name = stem.to_string();
        }
    }
    percent_decode_path_segment(&name)
}

fn percent_decode_path_segment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_string())
}

fn x_post_media_image(html: &str) -> Option<String> {
    if looks_like_x_login_page(html) {
        return None;
    }
    for property in ["og:image", "og:image:secure_url", "twitter:image"] {
        if let Some(url) = meta_content(html, property) {
            if is_x_post_media_url(&url) {
                return Some(url);
            }
        }
    }
    None
}

fn x_post_text(html: &str) -> Option<String> {
    if looks_like_x_login_page(html) {
        return None;
    }
    ["og:description", "twitter:description"]
        .into_iter()
        .filter_map(|property| meta_content(html, property))
        .map(|text| text.trim().to_string())
        .find(|text| {
            !text.is_empty()
                && !text.to_ascii_lowercase().contains("log in")
                && !text.to_ascii_lowercase().contains("sign up")
                && !text.to_ascii_lowercase().contains("see new posts")
        })
}

fn looks_like_x_login_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    [
        "login to x",
        "log in to x",
        "sign up for x",
        "see what's happening in the world right now",
        "see what’s happening in the world right now",
        "javascript is not available",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn looks_like_access_challenge(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    [
        "captcha",
        "cf-chl-",
        "checking your browser",
        "access denied",
        "enable javascript to continue",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_x_post_media_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    parsed.scheme() == "https" && is_x_media_host(&parsed) && parsed.path().contains("/media/")
}

fn x_title_from_url(url: &Url) -> Option<String> {
    let handle = url
        .path_segments()?
        .next()
        .filter(|handle| !handle.is_empty())?;
    Some(format!("X post by {handle}"))
}

fn read_bounded(response: Response, limit: u64, label: &str) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(format!(
            "{label} exceeds the {} MiB capture limit",
            limit / 1024 / 1024
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{label} could not be read: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{label} exceeds the {} MiB capture limit",
            limit / 1024 / 1024
        ));
    }
    Ok(bytes)
}

fn meta_content(html: &str, property: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(relative_start) = lower[cursor..].find("<meta") {
        let start = cursor + relative_start;
        let end = start + lower[start..].find('>')? + 1;
        let tag = &html[start..end];
        let marker = attribute(tag, "property").or_else(|| attribute(tag, "name"));
        if marker
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(property))
        {
            return attribute(tag, "content").map(decode_html_attribute);
        }
        cursor = end;
    }
    None
}

#[cfg(test)]
fn original_wikimedia_image(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("fullmedia")?;
    let end = (start + 64_000).min(html.len());
    let section = &html[start..end];
    let mut thumbnail = None;
    for tag in section.split('<') {
        let href = match attribute(tag, "href") {
            Some(href) if href.contains("upload.wikimedia.org") => decode_html_attribute(href),
            _ => continue,
        };
        if href.contains("/thumb/") {
            thumbnail.get_or_insert(href);
        } else {
            return Some(href);
        }
    }
    thumbnail.map(unwrap_commons_thumb)
}

fn unwrap_commons_thumb(url: String) -> String {
    let Some((prefix, rest)) = url.split_once("/thumb/") else {
        return url;
    };
    let parts: Vec<&str> = rest.split('/').take(3).collect();
    if parts.len() < 3 {
        return url;
    }
    format!("{prefix}/{}/{}/{}", parts[0], parts[1], parts[2])
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let start = lower.find(&format!("{name}="))? + name.len() + 1;
    let value = &tag[start..];
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return value.split_whitespace().next().map(str::to_string);
    }
    let value = &value[quote.len_utf8()..];
    let end = value.find(quote)?;
    Some(value[..end].to_string())
}

fn decode_html_attribute(value: String) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_article_text_with_entities_without_page_chrome() {
        let html = r#"<html><head><title>Archive &amp; Memory</title><script>const fake = '<article>wrong</article>';</script></head>
          <body><nav>Account Pricing Search</nav><article><h1>A short post</h1><p>Keep the user&#39;s idea &amp; its source.</p><blockquote>Even if the page disappears.</blockquote></article><footer>Legal</footer></body></html>"#;
        assert_eq!(html_title(html).as_deref(), Some("Archive & Memory"));
        assert_eq!(
            cleaned_main_text(html),
            "A short post\n\nKeep the user's idea & its source.\n\nEven if the page disappears."
        );
    }

    #[test]
    fn preserves_a_valid_short_post() {
        assert_eq!(
            cleaned_main_text("<main><p>Small idea, worth keeping.</p></main>"),
            "Small idea, worth keeping."
        );
    }

    #[test]
    fn x_idea_extraction_requires_public_post_text_metadata() {
        let public =
            r#"<meta property="og:description" content="A compact idea worth preserving.">"#;
        assert_eq!(
            x_post_text(public).as_deref(),
            Some("A compact idea worth preserving.")
        );
        assert_eq!(
            x_post_text(r#"<meta property="og:description" content="Log in to see new posts">"#),
            None
        );
        assert_eq!(
            x_post_text("<body>Navigation and replies only</body>"),
            None
        );
        let login_with_media = r#"<title>Log in to X</title><meta property="og:image" content="https://pbs.twimg.com/media/example.jpg">"#;
        assert_eq!(x_post_media_image(login_with_media), None);
    }

    #[test]
    fn generic_idea_urls_reject_local_and_private_literal_hosts() {
        for url in [
            "https://localhost/post",
            "https://127.0.0.1/post",
            "https://10.0.0.2/post",
            "https://[::1]/post",
            "https://[fc00::1]/post",
            "https://[fe80::1]/post",
            "https://[::ffff:127.0.0.1]/post",
            "https://[::ffff:10.0.0.1]/post",
        ] {
            assert!(!is_public_https_url(&Url::parse(url).unwrap()), "{url}");
        }
        assert!(is_public_https_url(
            &Url::parse("https://example.com/post").unwrap()
        ));
    }

    #[test]
    fn reads_wikimedia_title_and_original_media_from_page_markup() {
        let html = r#"<meta content="The Great Wave - Wikimedia Commons" property="og:title">
          <div class="fullMedia"><a href="//upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg">Original file</a></div>"#;
        assert_eq!(
            meta_content(html, "og:title").as_deref(),
            Some("The Great Wave - Wikimedia Commons")
        );
        assert_eq!(
            original_wikimedia_image(html).as_deref(),
            Some("//upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg")
        );
    }

    #[test]
    fn selects_the_largest_wikimedia_preview_when_the_original_exceeds_the_byte_budget() {
        let html = r#"<meta property="og:title" content="The Four Horsemen (CBL WEp 0021) - Wikimedia Commons">
          <div class="fullMedia"><a href="https://upload.wikimedia.org/wikipedia/commons/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg">Original file</a></div>
          <div class="mw-filepage-resolutioninfo">
            <a href="https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/500px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg">500 × 701</a>
            <a href="https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/960px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg">960 × 1,346</a>
            <a href="https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/1920px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg">1,920 × 2,692</a>
            <a href="https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/3840px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg">3,840 × 5,384</a>
          </div>"#;
        let candidates = wikimedia_image_candidates(html);
        assert_eq!(
            candidates[0],
            "https://upload.wikimedia.org/wikipedia/commons/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg"
        );
        assert!(candidates[1].contains("3840px-"));
        let sizes = [
            (
                "https://upload.wikimedia.org/wikipedia/commons/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg",
                53_297_493_u64,
            ),
            (
                "https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/3840px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg",
                11_530_803,
            ),
            (
                "https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/1920px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg",
                3_001_120,
            ),
        ];
        let chosen = first_in_budget(&candidates, MAX_IMAGE_BYTES, |url| {
            sizes
                .iter()
                .find(|(candidate, _)| *candidate == url)
                .map(|(_, length)| *length)
        })
        .expect("in-budget preview");
        assert!(chosen.contains("/thumb/"), "{chosen}");
        assert!(chosen.contains("3840px-"), "{chosen}");
    }

    #[test]
    fn synthesizes_an_in_budget_commons_preview_url_from_the_original_upload() {
        let original = Url::parse(
            "https://upload.wikimedia.org/wikipedia/commons/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg",
        )
        .unwrap();
        assert_eq!(
            wikimedia_thumb_url(&original, 3840).unwrap().as_str(),
            "https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/3840px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg"
        );
    }

    #[test]
    fn reads_public_x_post_media_and_title_without_logged_in_scraping() {
        let html = r#"<meta property="og:title" content="soli (@solisolsoli) on X">
          <meta property="og:image" content="https://pbs.twimg.com/media/HQmfIJJW0AEx2GZ.jpg:large">"#;
        assert_eq!(
            x_post_media_image(html).as_deref(),
            Some("https://pbs.twimg.com/media/HQmfIJJW0AEx2GZ.jpg:large")
        );
        assert_eq!(
            x_title_from_url(
                &Url::parse("https://x.com/solisolsoli/status/2092378489093595354").unwrap()
            )
            .as_deref(),
            Some("X post by solisolsoli")
        );
        let profile_only = r#"<meta property="og:image" content="https://pbs.twimg.com/profile_images/123/avatar.jpg">"#;
        assert_eq!(x_post_media_image(profile_only), None);
    }

    #[test]
    fn strips_preview_prefixes_from_preserved_file_names() {
        let thumb = Url::parse(
            "https://upload.wikimedia.org/wikipedia/commons/thumb/7/7c/The_Four_Horsemen_%28CBL_WEp_0021%29.jpg/3840px-The_Four_Horsemen_%28CBL_WEp_0021%29.jpg",
        )
        .unwrap();
        assert_eq!(
            image_file_name(&thumb),
            "The_Four_Horsemen_(CBL_WEp_0021).jpg"
        );
        let tweet = Url::parse("https://pbs.twimg.com/media/HQmfIJJW0AEx2GZ.jpg:large").unwrap();
        assert_eq!(image_file_name(&tweet), "HQmfIJJW0AEx2GZ.jpg");
    }

    #[test]
    fn generic_pages_request_manual_fallback_without_network_extraction() {
        let extractor = BoundedSourceExtractor::new().expect("create extractor");
        let result = extractor.extract(SourceExtractionRequest {
            source_link: "https://example.com/essay".to_string(),
        });
        assert_eq!(
            result,
            SourceExtraction::NeedsManualFallback {
                reason: "This source is not supported by bounded automatic extraction".to_string(),
            }
        );
    }

    #[test]
    fn wikipedia_article_hosts_are_outside_the_wikimedia_media_boundary() {
        let extractor = BoundedSourceExtractor::new().expect("create extractor");
        let result = extractor.extract(SourceExtractionRequest {
            source_link: "https://en.wikipedia.org/wiki/The_Great_Wave_off_Kanagawa".to_string(),
        });
        assert_eq!(
            result,
            SourceExtraction::NeedsManualFallback {
                reason: "This source is not supported by bounded automatic extraction".to_string(),
            }
        );
    }

    #[test]
    fn non_https_and_credentialed_links_request_manual_fallback() {
        let extractor = BoundedSourceExtractor::new().expect("create extractor");
        assert_eq!(
            extractor.extract(SourceExtractionRequest {
                source_link: "http://commons.wikimedia.org/wiki/File:The_Great_Wave.jpg"
                    .to_string(),
            }),
            SourceExtraction::NeedsManualFallback {
                reason: "Use a public HTTPS source link for automatic capture".to_string(),
            }
        );
        assert_eq!(
            extractor.extract(SourceExtractionRequest {
                source_link: "https://user@commons.wikimedia.org/wiki/File:The_Great_Wave.jpg"
                    .to_string(),
            }),
            SourceExtraction::NeedsManualFallback {
                reason: "Use a public HTTPS source link for automatic capture".to_string(),
            }
        );
    }

    #[test]
    fn wikimedia_redirects_stay_on_supported_hosts_and_stop_after_four_hops() {
        let commons = Url::parse("https://commons.wikimedia.org/wiki/File:Wave.jpg").unwrap();
        let upload = Url::parse("https://upload.wikimedia.org/wikipedia/commons/a.jpg").unwrap();
        let x = Url::parse("https://x.com/example/status/1").unwrap();
        let media = Url::parse("https://pbs.twimg.com/media/example.jpg:large").unwrap();
        let other = Url::parse("https://example.com/file.jpg").unwrap();
        assert!(should_follow_supported_redirect(
            &[commons.clone()],
            &upload
        ));
        assert!(!should_follow_supported_redirect(&[commons.clone()], &x));
        assert!(!should_follow_supported_redirect(&[x.clone()], &media));
        assert!(should_follow_supported_redirect(&[media.clone()], &media));
        assert!(!should_follow_supported_redirect(
            &[commons.clone(), commons.clone(), commons.clone(), commons],
            &upload
        ));
        assert!(!should_follow_supported_redirect(&[x], &other));
    }

    #[test]
    fn falls_back_to_open_graph_image_when_full_media_is_absent() {
        let html = r#"<meta property="og:image" content="https://upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg">"#;
        assert_eq!(original_wikimedia_image(html), None);
        assert_eq!(
            meta_content(html, "og:image")
                .map(unwrap_commons_thumb)
                .as_deref(),
            Some("https://upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg")
        );
    }

    #[test]
    fn prefers_the_original_upload_over_a_thumbnail_in_full_media() {
        let html = r#"<div class="fullMedia"><a href="//upload.wikimedia.org/wikipedia/commons/thumb/a/a5/Great_Wave.jpg/800px-Great_Wave.jpg">thumb</a><a href="//upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg">Original file</a></div>"#;
        assert_eq!(
            original_wikimedia_image(html).as_deref(),
            Some("//upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg")
        );
    }

    #[test]
    fn unwraps_commons_thumbnail_urls_to_the_original_file() {
        assert_eq!(
            unwrap_commons_thumb(
                "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a5/Great_Wave.jpg/800px-Great_Wave.jpg"
                    .to_string()
            ),
            "https://upload.wikimedia.org/wikipedia/commons/a/a5/Great_Wave.jpg"
        );
    }
}
