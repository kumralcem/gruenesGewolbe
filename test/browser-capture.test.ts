import { test } from "node:test";
import assert from "node:assert/strict";
import { validateBrowserCapture } from "../src/browser-capture.ts";
const source = {
  version: 1,
  intent: "idea",
  url: "https://example.org/private",
  title: "Private article",
  capturedAt: "2026-09-11T10:00:00Z",
  text: "Visible article text",
};
test("browser input carries content without accepting credentials or configuration", () => {
  const result = validateBrowserCapture({
    ...source,
    cookies: "SECRET",
    headers: { authorization: "secret" },
    config: { apiKey: "secret" },
  });
  assert.equal(result.text, source.text);
  assert.equal("cookies" in result, false);
  assert.equal("config" in result, false);
  assert.equal("headers" in result, false);
});
test("visual captures require bytes for exactly one selected image", () => {
  assert.throws(
    () => validateBrowserCapture({ ...source, intent: "art" }),
    /Select one/,
  );
  assert.throws(
    () => validateBrowserCapture({ ...source, url: "file:///etc/passwd" }),
    /Invalid page/,
  );
});
test("an unclassified snapshot preserves multiple images, instructions and missing media", () => {
  const capture = validateBrowserCapture({
    ...source,
    version: 2,
    intent: undefined,
    instructions: "Save each painting separately",
    images: [
      {
        url: "https://example.org/a.jpg",
        bytes: "/9j/",
        mimeType: "image/jpeg",
        alt: "First",
        caption: "Artist A",
      },
      {
        url: "https://example.org/b.jpg",
        error: "Download unavailable",
        alt: "Second",
      },
    ],
    cookies: "secret",
  });
  assert.equal(capture.intent, "capture");
  assert.equal(capture.instructions, "Save each painting separately");
  assert.equal(capture.images?.length, 2);
  assert.equal(capture.images?.[0].caption, "Artist A");
  assert.equal(capture.images?.[1].error, "Download unavailable");
  assert.equal("cookies" in capture, false);
  assert.throws(
    () =>
      validateBrowserCapture({
        ...capture,
        images: Array(25).fill(capture.images![0]),
      }),
    /24/,
  );
});
