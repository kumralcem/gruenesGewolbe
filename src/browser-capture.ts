import { originalTextFile, validateImportedText } from "./text-import.ts";
import { createHash } from "node:crypto";
import { isLocalSource } from "./local-source.ts";
export interface CaptureImage {
  url: string;
  bytes?: string;
  mimeType?: string;
  alt?: string;
  caption?: string;
  width?: number;
  height?: number;
  error?: string;
}
export interface BrowserCapture {
  document?: { extension: string };
  version: 1 | 2;
  intent: "art" | "idea" | "capture";
  instructions?: string;
  contextText?: string;
  images?: CaptureImage[];
  warnings?: string[];
  url: string;
  title: string;
  capturedAt: string;
  text: string;
  html?: string;
  transcript?: string;
  focus?: string;
  image?: { url: string; bytes: string; mimeType: string };
}
function string(value: unknown, max: number) {
  return typeof value === "string" && value.length <= max;
}
export function validateBrowserCapture(input: unknown): BrowserCapture {
  const b = input as BrowserCapture;
  if (
    !b ||
    ![1, 2].includes(b.version) ||
    (b.version === 1 && !["art", "idea"].includes(b.intent)) ||
    !string(b.url, 4000) ||
    !string(b.title, 1000) ||
    !string(b.text, 400000) ||
    !string(b.capturedAt, 100) ||
    !Number.isFinite(Date.parse(b.capturedAt)) ||
    !string(b.html ?? "", 1500000) ||
    !string(b.transcript ?? "", 400000) ||
    !string(b.focus ?? "", 2000) ||
    !string(b.instructions ?? "", 2000) ||
    !string(b.contextText ?? "", 40000)
  )
    throw Error("Invalid browser capture");
  const url = new URL(b.url);
  if (
    (!["https:", "http:"].includes(url.protocol) && !isLocalSource(b.url)) ||
    url.username ||
    url.password
  )
    throw Error("Invalid page URL");
  if (b.version === 2) {
    if (!Array.isArray(b.images) || b.images.length > 24)
      throw Error("Capture accepts at most 24 images");
    let size = 0;
    const images = b.images.map((image): CaptureImage => {
      if (
        !image ||
        !string(image.url, 4000) ||
        !string(image.alt ?? "", 1000) ||
        !string(image.caption ?? "", 4000) ||
        !string(image.error ?? "", 1000)
      )
        throw Error("Invalid image candidate");
      const u = new URL(image.url);
      if (
        (!["http:", "https:"].includes(u.protocol) &&
          !(isLocalSource(b.url) && image.url === b.url)) ||
        u.username ||
        u.password
      )
        throw Error("Invalid image reference");
      if (
        image.bytes !== undefined &&
        (!string(image.bytes, 86000000) ||
          !image.bytes ||
          !/^[A-Za-z0-9+/]*={0,2}$/.test(image.bytes) ||
          !["image/jpeg", "image/png", "image/webp"].includes(
            image.mimeType ?? "",
          ))
      )
        throw Error("Invalid captured image bytes");
      size += image.bytes?.length ?? 0;
      if (size > 86000000) throw Error("Captured images exceed 64 MB");
      return {
        url: image.url,
        bytes: image.bytes,
        mimeType: image.mimeType,
        alt: image.alt,
        caption: image.caption,
        error:
          image.error ?? (image.bytes ? undefined : "Image bytes unavailable"),
      };
    });
    if (!b.text.trim() && !b.transcript?.trim() && !images.some((i) => i.bytes))
      throw Error("The page contains no readable content");
    if (
      b.warnings !== undefined &&
      (!Array.isArray(b.warnings) ||
        b.warnings.length > 30 ||
        !b.warnings.every((w) => string(w, 1000)))
    )
      throw Error("Invalid capture warnings");
    if (b.document !== undefined) {
      if (
        !isLocalSource(b.url) ||
        !b.document ||
        typeof b.document.extension !== "string" ||
        images.length ||
        b.html ||
        b.transcript ||
        b.contextText
      )
        throw Error("Invalid local document capture");
      originalTextFile(b.document.extension);
      validateImportedText(b.text);
      if (
        "gg-local:sha256:" +
          createHash("sha256").update(b.text, "utf8").digest("hex") !==
        b.url
      )
        throw Error("Local document must match its content hash");
    } else if (isLocalSource(b.url)) {
      const image = images[0];
      if (
        images.length !== 1 ||
        !image.bytes ||
        image.url !== b.url ||
        "gg-local:sha256:" +
          createHash("sha256")
            .update(Buffer.from(image.bytes, "base64"))
            .digest("hex") !==
          b.url
      )
        throw Error("Local import must contain its matching original image");
    }
    return {
      document: b.document ? { extension: b.document.extension } : undefined,
      version: 2,
      intent: "capture",
      url: b.url,
      title: b.title,
      capturedAt: b.capturedAt,
      text: b.text,
      html: b.html,
      transcript: b.transcript,
      instructions: b.instructions || b.focus || undefined,
      contextText: b.contextText,
      images,
      warnings: b.warnings,
    };
  }
  if (isLocalSource(b.url))
    throw Error("Local imports require capture version 2");
  if (b.intent === "art") {
    if (
      !b.image ||
      !string(b.image.url, 4000) ||
      !string(b.image.bytes, 86000000) ||
      !["image/jpeg", "image/png", "image/webp"].includes(b.image.mimeType) ||
      !b.image.bytes ||
      !/^[A-Za-z0-9+/]*={0,2}$/.test(b.image.bytes)
    )
      throw Error("Select one supported image and include its bytes");
    const imageUrl = new URL(b.image.url);
    if (
      !["https:", "http:"].includes(imageUrl.protocol) ||
      imageUrl.username ||
      imageUrl.password
    )
      throw Error("Invalid selected-image reference");
  }
  if (b.intent === "idea" && !b.text.trim() && !b.transcript?.trim())
    throw Error("The page contains no readable text");
  // Project permitted fields. Cookies, request headers, form values and browser
  // storage are not accepted as worker input, even if a caller includes them.
  return {
    version: 1,
    intent: b.intent,
    url: b.url,
    title: b.title,
    capturedAt: b.capturedAt,
    text: b.text,
    html: b.html,
    transcript: b.transcript,
    focus: b.focus,
    image:
      b.intent === "art"
        ? {
            url: b.image!.url,
            bytes: b.image!.bytes,
            mimeType: b.image!.mimeType,
          }
        : undefined,
  };
}
