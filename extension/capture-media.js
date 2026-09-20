async function readImage(url, tabId) {
  if (
    /^https?:/.test(url) &&
    (await chrome.permissions.contains({
      origins: [new URL(url).origin + "/*"],
    }))
  ) {
    const response = await fetch(url, {
      credentials: "include",
      signal: AbortSignal.timeout(15000),
      cache: "force-cache",
    });
    return await encodeResponse(response);
  }
  const [{ result }] = await chrome.scripting.executeScript({
    target: { tabId },
    func: async (url) => {
      if (
        !Array.from(document.images).some(
          (i) => (i.currentSrc || i.src) === url,
        )
      )
        return {
          error:
            "The selected image is no longer on the source page. Capture it again.",
        };
      try {
        const response = await fetch(url, {
          credentials: "include",
          signal: AbortSignal.timeout(15000),
          cache: "force-cache",
        });
        if (!response.ok) throw Error("Image returned HTTP " + response.status);
        const reader = response.body.getReader(),
          chunks = [];
        let size = 0;
        while (true) {
          const part = await reader.read();
          if (part.done) break;
          size += part.value.length;
          if (size > 64000000) {
            await reader.cancel();
            throw Error("Image exceeds 64 MB");
          }
          chunks.push(part.value);
        }
        const blob = new Blob(chunks, {
          type: response.headers.get("content-type")?.split(";")[0] ?? "",
        });
        const bytes = await new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result).split(",")[1]);
          reader.onerror = reject;
          reader.readAsDataURL(blob);
        });
        return { bytes, mimeType: blob.type };
      } catch (e) {
        return { error: e.message };
      }
    },
    args: [url],
  });
  if (result.error) throw Error(result.error);
  return result;
}
async function encodeResponse(response) {
  if (!response.ok) throw Error("Image returned HTTP " + response.status);
  const reader = response.body.getReader();
  const chunks = [];
  let size = 0;
  while (true) {
    const part = await reader.read();
    if (part.done) break;
    size += part.value.length;
    if (size > 64000000) {
      await reader.cancel();
      throw Error("Image exceeds 64 MB");
    }
    chunks.push(part.value);
  }
  const blob = new Blob(chunks, {
    type: response.headers.get("content-type") ?? "",
  });
  const data = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (let offset = 0; offset < data.length; offset += 32768)
    binary += String.fromCharCode(...data.subarray(offset, offset + 32768));
  return { bytes: btoa(binary), mimeType: blob.type.split(";")[0] };
}

export async function prepareCapture(snapshot, tabId, instructions) {
  const images = [];
  const warnings = [...(snapshot.warnings ?? [])];
  let bytes = 0;
  for (const image of snapshot.images.slice(0, 24)) {
    let file;
    try {
      file = await readImage(image.url, tabId);
      if (!["image/jpeg", "image/png", "image/webp"].includes(file.mimeType))
        throw Error("Unsupported image format");
      if (bytes + file.bytes.length > 80000000)
        throw Error("Capture image size limit reached");
      bytes += file.bytes.length;
    } catch (e) {
      file = { error: e.message };
    }
    let url = image.url;
    if (!/^https?:/.test(url))
      url = snapshot.url.split("#")[0] + "#gg-image-" + images.length;
    images.push({ url, alt: image.alt, caption: image.caption, ...file });
  }
  if (snapshot.images.length > 24)
    warnings.push(
      "Only the first 24 relevant image candidates were collected.",
    );
  return {
    version: 2,
    url: snapshot.url,
    title: snapshot.title,
    capturedAt: snapshot.capturedAt,
    text: snapshot.text,
    html: snapshot.html,
    transcript: snapshot.transcript,
    contextText: snapshot.contextText,
    instructions: instructions || undefined,
    images,
    warnings,
  };
}
