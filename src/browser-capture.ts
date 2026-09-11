export interface BrowserCapture {
  version: 1;
  intent: "art" | "idea";
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
    b.version !== 1 ||
    !["art", "idea"].includes(b.intent) ||
    !string(b.url, 4000) ||
    !string(b.title, 1000) ||
    !string(b.text, 400000) ||
    !string(b.capturedAt, 100) ||
    !Number.isFinite(Date.parse(b.capturedAt)) ||
    !string(b.html ?? "", 1500000) ||
    !string(b.transcript ?? "", 400000) ||
    !string(b.focus ?? "", 2000)
  )
    throw Error("Invalid browser capture");
  const url = new URL(b.url);
  if (
    !["https:", "http:"].includes(url.protocol) ||
    url.username ||
    url.password
  )
    throw Error("Invalid page URL");
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
