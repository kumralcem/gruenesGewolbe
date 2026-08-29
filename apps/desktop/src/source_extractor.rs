use std::io::Read;
use std::time::Duration;

use gruenes_gewolbe_core::{SourceExtraction, SourceExtractionRequest, SourceExtractor};
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::Url;

const MAX_PAGE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;

pub struct BoundedSourceExtractor {
    client: Client,
}

impl BoundedSourceExtractor {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("GruenesGewoelbe/0.1 personal archive")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if should_follow_redirect(attempt.previous().len(), attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| format!("source extractor could not start: {error}"))?;
        Ok(Self { client })
    }

    fn extract_wikimedia(&self, source: &Url) -> Result<SourceExtraction, String> {
        if source.host_str() == Some("upload.wikimedia.org") {
            return self.download_image(source.clone(), None);
        }

        let page = self
            .client
            .get(source.clone())
            .send()
            .and_then(Response::error_for_status)
            .map_err(|error| format!("Wikimedia page could not be read: {error}"))?;
        let html = read_bounded(page, MAX_PAGE_BYTES, "Wikimedia page")?;
        let html =
            String::from_utf8(html).map_err(|_| "Wikimedia page was not UTF-8 text".to_string())?;
        let title = meta_content(&html, "og:title")
            .map(|value| value.trim_end_matches(" - Wikimedia Commons").to_string());
        let image_link = original_wikimedia_image(&html)
            .or_else(|| meta_content(&html, "og:image").map(unwrap_commons_thumb))
            .ok_or_else(|| "Wikimedia page did not expose a preserved image".to_string())?;
        let image_url = if image_link.starts_with("//") {
            Url::parse(&format!("https:{image_link}"))
        } else {
            source.join(&image_link)
        }
        .map_err(|_| "Wikimedia image link was invalid".to_string())?;
        if image_url.host_str() != Some("upload.wikimedia.org") {
            return Err("Wikimedia image link left the supported media host".to_string());
        }
        self.download_image(image_url, title)
    }

    fn download_image(
        &self,
        image_url: Url,
        title: Option<String>,
    ) -> Result<SourceExtraction, String> {
        let response = self
            .client
            .get(image_url.clone())
            .send()
            .and_then(Response::error_for_status)
            .map_err(|error| format!("Wikimedia image could not be downloaded: {error}"))?;
        let is_image = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("image/"));
        if !is_image {
            return Err("Wikimedia response was not an image".to_string());
        }
        let file_name = image_url
            .path_segments()
            .and_then(|segments| segments.last())
            .filter(|name| !name.is_empty())
            .unwrap_or("wikimedia-image")
            .to_string();
        let title = title.or_else(|| {
            file_name
                .rsplit_once('.')
                .map(|(stem, _)| stem)
                .or(Some(file_name.as_str()))
                .map(|value| value.replace('_', " "))
        });
        let bytes = read_bounded(response, MAX_IMAGE_BYTES, "Wikimedia image")?;
        Ok(SourceExtraction::ExtractedImage {
            title,
            file_name,
            bytes,
        })
    }
}

impl SourceExtractor for BoundedSourceExtractor {
    fn extract(&self, request: SourceExtractionRequest) -> SourceExtraction {
        let source = match Url::parse(&request.source_link) {
            Ok(source) if source.scheme() == "https" && source.username().is_empty() => source,
            _ => return fallback("Use a public HTTPS source link for automatic capture"),
        };
        let host = source.host_str().unwrap_or_default();
        if matches!(
            host,
            "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com"
        ) {
            return fallback(
                "X.com requires pasted content because logged-in extraction is not attempted",
            );
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

fn should_follow_redirect(previous: usize, url: &Url) -> bool {
    previous < 4 && is_wikimedia_host(url)
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
    fn x_links_request_manual_fallback_without_attempting_logged_in_extraction() {
        let extractor = BoundedSourceExtractor::new().expect("create extractor");
        let result = extractor.extract(SourceExtractionRequest {
            source_link: "https://x.com/example/status/123".to_string(),
        });
        assert_eq!(
            result,
            SourceExtraction::NeedsManualFallback {
                reason:
                    "X.com requires pasted content because logged-in extraction is not attempted"
                        .to_string(),
            }
        );
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
        let other = Url::parse("https://example.com/file.jpg").unwrap();
        assert!(should_follow_redirect(0, &commons));
        assert!(should_follow_redirect(3, &upload));
        assert!(!should_follow_redirect(4, &commons));
        assert!(!should_follow_redirect(0, &other));
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
