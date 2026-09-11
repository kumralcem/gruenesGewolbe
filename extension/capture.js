const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search),
  id = params.get("id");
let snapshot, tabId, prepared, allowedHost;
const settings = await chrome.storage.local.get(["receiver", "token"]);
$("receiver").value = settings.receiver ?? "http://127.0.0.1:48123";
$("token").value = settings.token ?? "";
if (!settings.token) $("settings").open = true;
function connection() {
  const url = new URL($("receiver").value);
  if (
    url.protocol !== "http:" ||
    url.hostname !== "127.0.0.1" ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  )
    throw Error("Use the local http://127.0.0.1 address printed by GG.");
  return url.origin;
}
async function request(path, options = {}) {
  const response = await fetch(connection() + path, {
    ...options,
    headers: {
      authorization: "Bearer " + $("token").value,
      ...options.headers,
    },
  });
  const result = await response.json();
  if (!response.ok) throw Error(result.error ?? "GG request failed");
  return result;
}
function status(text) {
  $("status").textContent = text;
}
function updateIntent() {
  const art = document.querySelector("[name=intent]:checked").value === "art";
  $("images").hidden = !art;
  $("summary").textContent = art
    ? "Choose exactly the image you want to keep. Surrounding discussion will not be saved."
    : snapshot?.transcript
      ? "The open transcript will be preserved and summarized."
      : snapshot?.selection
        ? "Your selected text will be preserved and summarized."
        : "The page text will be preserved with a searchable summary.";
}
if (params.has("error")) {
  status(params.get("error"));
  $("capture").disabled = true;
} else if (id) {
  const entry = (await chrome.storage.session.get("capture_" + id))[
    "capture_" + id
  ];
  if (!entry) {
    status("This capture has expired. Click GG on the source page again.");
    $("capture").disabled = true;
  } else {
    ({ snapshot, tabId } = entry);
    $("source").textContent = snapshot.title + " — " + snapshot.url;
    for (const [index, image] of snapshot.images.entries()) {
      const label = document.createElement("label");
      label.className = "image-option";
      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "image";
      radio.value = String(index);
      radio.checked = snapshot.images.length === 1;
      radio.addEventListener("change", () => {
        prepared = undefined;
        $("allow").hidden = true;
      });
      const preview = document.createElement("img");
      preview.src = image.url;
      preview.alt = image.alt || "Image " + (index + 1);
      const text = document.createElement("span");
      text.textContent = `${image.width} × ${image.height} · ${image.alt || "Image " + (index + 1)}`;
      label.append(radio, preview, text);
      $("images").append(label);
    }
    updateIntent();
  }
} else {
  $("capture").disabled = true;
  status("Click the GG toolbar icon on a page to start a capture.");
}
document.querySelectorAll("[name=intent]").forEach((input) =>
  input.addEventListener("change", () => {
    prepared = undefined;
    updateIntent();
  }),
);
$("focus").addEventListener("input", () => {
  prepared = undefined;
});
async function readImage(url) {
  if (allowedHost === new URL(url).origin) {
    const response = await fetch(url, {
      credentials: "include",
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
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () =>
      resolve({
        bytes: String(r.result).split(",")[1],
        mimeType: blob.type.split(";")[0],
      });
    r.onerror = reject;
    r.readAsDataURL(blob);
  });
}
async function send() {
  $("capture").disabled = true;
  try {
    const intent = document.querySelector("[name=intent]:checked").value;
    const selected = document.querySelector("[name=image]:checked");
    if (intent === "art" && !selected) throw Error("Choose one image first.");
    status("Preparing capture…");
    if (!prepared) {
      const image =
        intent === "art" ? snapshot.images[Number(selected.value)] : undefined;
      let file;
      if (image) {
        try {
          file = await readImage(image.url);
        } catch (error) {
          if (/^https?:/.test(image.url)) $("allow").hidden = false;
          throw Error("Could not read the selected image: " + error.message);
        }
        if (!["image/jpeg", "image/png", "image/webp"].includes(file.mimeType))
          throw Error("This image format is not supported yet.");
      }
      const imageUrl =
        image &&
        (/^https?:/.test(image.url)
          ? image.url
          : snapshot.url.split("#")[0] +
            "#gg-image-" +
            Array.from(
              new Uint8Array(
                await crypto.subtle.digest(
                  "SHA-256",
                  Uint8Array.from(atob(file.bytes), (c) => c.charCodeAt(0)),
                ),
              ),
            )
              .map((b) => b.toString(16).padStart(2, "0"))
              .join(""));
      prepared = {
        version: 1,
        intent,
        url: snapshot.url,
        title: snapshot.title,
        capturedAt: snapshot.capturedAt,
        text: snapshot.text,
        html: snapshot.html,
        transcript: snapshot.transcript,
        focus: $("focus").value,
        image: image ? { url: imageUrl, ...file } : undefined,
      };
    }
    const job = await request("/captures", {
      method: "POST",
      headers: { "content-type": "application/json", "x-gg-capture-id": id },
      body: JSON.stringify(prepared),
    });
    status(
      "Capture accepted. You can close this tab; GG will continue processing.",
    );
    document
      .querySelectorAll("[name=intent],[name=image],#focus")
      .forEach((input) => (input.disabled = true));
    await chrome.storage.session.remove("capture_" + id);
    await refresh();
    await poll(job.id).catch((error) => {
      $("connection").textContent =
        "Capture accepted; status unavailable: " + error.message;
    });
  } catch (error) {
    status(error.message);
    $("capture").disabled = false;
  }
}
async function refresh() {
  try {
    const jobs = await request("/captures");
    $("jobs").replaceChildren();
    for (const job of jobs) {
      const li = document.createElement("li");
      li.textContent =
        job.title +
        " — " +
        (["pending", "running"].includes(job.status)
          ? job.status
          : (job.result?.outcome?.status ?? job.status));
      const detail = document.createElement("small");
      detail.textContent =
        job.result?.outcome?.path ??
        job.result?.outcome?.reason ??
        job.error ??
        job.url;
      li.append(detail);
      if (["failed", "interrupted"].includes(job.status)) {
        const retry = document.createElement("button");
        retry.className = "secondary";
        retry.textContent = "Retry";
        retry.onclick = async () => {
          try {
            await request("/captures/" + job.id + "/retry", { method: "POST" });
            await refresh();
            await poll(job.id);
          } catch (e) {
            status(e.message);
          }
        };
        li.append(retry);
      }
      $("jobs").append(li);
    }
  } catch (error) {
    $("connection").textContent = error.message;
  }
}
async function poll(jobId) {
  for (let i = 0; i < 90; i++) {
    await new Promise((resolve) => setTimeout(resolve, 2000));
    const job = await request("/captures/" + jobId);
    if (!["pending", "running"].includes(job.status)) {
      await refresh();
      return;
    }
  }
}
$("capture").onclick = send;
$("refresh").onclick = refresh;
$("allow").onclick = async () => {
  try {
    const selected = document.querySelector("[name=image]:checked");
    if (!selected) throw Error("Choose one image.");
    const host =
      new URL(snapshot.images[Number(selected.value)].url).origin + "/*";
    if (await chrome.permissions.request({ origins: [host] })) {
      allowedHost = new URL(snapshot.images[Number(selected.value)].url).origin;
      prepared = undefined;
      $("allow").hidden = true;
      await send();
    } else status("Image host permission was not granted.");
  } catch (error) {
    status(error.message);
  }
};
$("connect").onclick = async () => {
  try {
    await request("/health");
    await chrome.storage.local.set({
      receiver: connection(),
      token: $("token").value,
    });
    $("connection").textContent = "Connected to GG.";
    $("settings").open = false;
    await refresh();
  } catch (error) {
    $("connection").textContent = error.message;
  }
};
if (settings.token) await refresh();
