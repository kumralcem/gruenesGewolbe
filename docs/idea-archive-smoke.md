# Idea archive acceptance

This is the native acceptance checklist for retiring browser tabs. Automated core, command, and mocked browser tests do not substitute for this run. Use a temporary Vault and the bounded verification procedure before launching; avoid concurrent builds/tests. Record actual outcomes below rather than checking off planned behavior.

## Capture and retrieval

1. Open the app and confirm Paintings and Idea Sources are independently visible in left navigation. Existing image import and browsing still work.
2. Enter Idea Sources. Paste a public blog/article URL. Confirm the result contains locally readable main text, not just the title, link, navigation, or an excerpt. Inspect the Item Folder's source copy and Markdown record.
3. Paste a tweet/post URL, including a text-only post. When public text is unavailable, confirm the app requests pasted source text and keeps the URL/draft. Paste the post text and save it. An image attached to a post must not redirect an idea capture into Paintings.
4. Capture a generic website and an unavailable/blocked URL. Missing, oversized, or incomplete extraction must not be presented as successfully archived content. The pasted-text path remains available.
5. Without provider configuration, confirm source capture succeeds and the UI clearly indicates the summary has not been generated. Configure a provider only if a live request is intended; verify credentials remain outside the Vault and are not displayed in records or logs.
6. With a configured provider, capture source text and confirm the app generates a summary of the source's central point. Read the text and summary separately. Cause a provider failure using a nonworking configuration; the already saved source remains readable, and retrying summary generation must not duplicate the item.
7. Edit a summary and save. Confirm later enrichment does not silently overwrite that edit. Test switching items/Vaults during a delayed operation: its completion must not appear on the wrong item or write to another Vault.
8. Close the app, disconnect networking, and reopen the Vault. Read the captured post, article, and website content and summaries. Search for their saved metadata/summary, open results, and confirm the source is available without the original site.

## Responsiveness

Use representative existing images, noting image sizes and Vault item count. Compare first and repeat selections, both during thumbnail preparation and after it finishes. Record click-to-visible-preview and click-to-details separately. The UI should acknowledge selection immediately and remain usable during background work; code-path improvements alone are not evidence of measured native latency.

## Evidence

- Date, platform, Vault size, and representative source types.
- Actual automatic-extraction versus manual-fallback outcomes.
- Local source/record inspection and offline restart result.
- Provider configuration, success/failure/retry outcome without any key value.
- Image selection measurements and resource use.
- Outstanding failures and exact reproduction steps.

Native execution status: not yet recorded.
