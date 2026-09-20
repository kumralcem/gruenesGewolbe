import { test } from "node:test";
import assert from "node:assert/strict";
import { sameCapturedSource } from "../src/source-url.ts";
test("browser source aliases retain video/post identity without matching unrelated pages", () => {
  assert.equal(
    sameCapturedSource(
      "https://x.com/user/status/123?s=20",
      "https://twitter.com/user/status/123",
    ),
    true,
  );
  assert.equal(
    sameCapturedSource(
      "https://www.youtube.com/watch?v=abcdefghijk&t=10",
      "https://youtu.be/abcdefghijk",
    ),
    true,
  );
  assert.equal(
    sameCapturedSource(
      "https://x.com/u/status/123",
      "https://x.com/u/status/124",
    ),
    false,
  );
  assert.equal(
    sameCapturedSource(
      "https://x.com/u/status/123",
      "https://fake.example/u/status/123",
    ),
    false,
  );
  assert.equal(
    sameCapturedSource(
      "https://example.com/?page=1",
      "https://example.com/?page=2",
    ),
    false,
  );
});
