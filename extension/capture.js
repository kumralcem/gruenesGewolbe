const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
let id = params.get("id"),
  snapshot,
  tabId,
  prepared;
const settings = await chrome.storage.local.get([
  "receiver",
  "token",
  "notifications",
]);
$("receiver").value = settings.receiver ?? "http://127.0.0.1:48123";
$("token").value = settings.token ?? "";
$("notifications").checked = Boolean(settings.notifications);
if (!settings.token) $("settings").open = true;
function connection() {
  const url = new URL($("receiver").value);
  if (
    (url.protocol !== "https:" &&
      !(url.protocol === "http:" && url.hostname === "127.0.0.1")) ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  )
    throw Error("Use an HTTPS GG address, or http://127.0.0.1 for local use.");
  return url.origin;
}
async function request(path, options = {}) {
  const response = await fetch(connection() + path, {
    ...options,
    redirect: "error",
    headers: {
      authorization: "Bearer " + $("token").value,
      ...options.headers,
    },
    signal: AbortSignal.timeout(120000),
  });
  const data = await response.json();
  if (!response.ok) throw Error(data.error ?? "GG request failed");
  return data;
}
function status(message) {
  $("status").textContent = message;
}
if (params.has("error")) {
  status(params.get("error"));
  $("capture").disabled = true;
} else if (id) {
  const entry = (await chrome.storage.session.get("capture_" + id))[
    "capture_" + id
  ];
  if (!entry) {
    status("This capture expired. Open GG on the source page again.");
    $("capture").disabled = true;
  } else {
    ({ snapshot, tabId } = entry);
    $("source").textContent = snapshot.title + " — " + snapshot.url;
    $("summary").textContent =
      "GG will interpret this page and collect relevant images. Instructions are optional.";
  }
} else {
  $("capture").disabled = true;
  status("Open GG on the page you want to capture.");
}
$("focus").oninput = () => {
  prepared = undefined;
};
async function readImage(url) {
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
    status("Collecting the page and images…");
    if (!prepared) {
      const images = [];
      const warnings = [...(snapshot.warnings ?? [])];
      let bytes = 0;
      for (const image of snapshot.images.slice(0, 24)) {
        let file;
        try {
          file = await readImage(image.url);
          if (
            !["image/jpeg", "image/png", "image/webp"].includes(file.mimeType)
          )
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
      prepared = {
        version: 2,
        url: snapshot.url,
        title: snapshot.title,
        capturedAt: snapshot.capturedAt,
        text: snapshot.text,
        html: snapshot.html,
        transcript: snapshot.transcript,
        contextText: snapshot.contextText,
        instructions: $("focus").value || undefined,
        images,
        warnings,
      };
    }
    const job = await request("/captures", {
      method: "POST",
      headers: { "content-type": "application/json", "x-gg-capture-id": id },
      body: JSON.stringify(prepared),
    });
    const missing = prepared.images.filter((i) => i.error).length;
    status(
      `Capture received. You can close this tab.${missing ? ` ${missing} image(s) unavailable; GG will preserve the available content.` : ""}`,
    );
    $("focus").disabled = true;
    $("allow").disabled = true;
    await chrome.storage.session.remove("capture_" + id);
    await refresh();
    await poll(job.id);
  } catch (e) {
    status(e.message);
    $("capture").disabled = false;
  }
}
async function refresh() {
  try {
    const [jobs, usage] = await Promise.all([
      request("/captures"),
      request("/usage"),
    ]);
    $("jobs").replaceChildren();
    for (const job of jobs) {
      const li = document.createElement("li");
      li.textContent =
        job.title + " — " + (job.result?.outcome?.status ?? job.status);
      const detail = document.createElement("small");
      detail.textContent =
        job.error ??
        job.result?.outcome?.reason ??
        job.result?.outcome?.path ??
        job.url;
      li.append(detail);
      if (
        ["failed", "interrupted", "partial", "paused", "cancelled"].includes(
          job.status,
        )
      ) {
        const retry = document.createElement("button");
        retry.className = "secondary";
        retry.textContent = "Retry unfinished work";
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
      if (["pending", "running", "paused"].includes(job.status)) {
        const cancel = document.createElement("button");
        cancel.className = "secondary";
        cancel.textContent = "Cancel";
        cancel.onclick = async () => {
          try {
            await request("/captures/" + job.id + "/cancel", {
              method: "POST",
            });
            await refresh();
          } catch (e) {
            status(e.message);
          }
        };
        li.append(cancel);
      }
      $("jobs").append(li);
    }
    const data = usage.usage ?? usage;
    const windows = (data.windows ?? [])
      .map(
        (w) =>
          `${w.name}: ${w.usedRequests}/${w.requests} requests, ${w.usedTokens}/${w.tokens} tokens`,
      )
      .join(" · ");
    const provider = usage.provider ?? "configured provider",
      allowance = data.providers?.[provider]?.allowance;
    const paused = data.providers?.[provider]?.paused;
    $("usage").textContent =
      `${provider} · ${windows} · Subscription allowance: ${allowance ? `${allowance.usedPercent}% used (last reported ${new Date(allowance.observedAt).toLocaleString()})` : "unavailable"}${paused ? " · Paused: " + paused : ""}${data.warning ? " · Approaching application limit" : ""}`;
    if (
      (data.warning || paused || allowance?.usedPercent >= 80) &&
      $("notifications").checked &&
      Notification.permission === "granted"
    ) {
      const key = JSON.stringify([
        data.warning,
        paused,
        allowance?.usedPercent,
      ]);
      if (sessionStorage.getItem("usageAlert") !== key) {
        new Notification("GG usage warning", { body: $("usage").textContent });
        sessionStorage.setItem("usageAlert", key);
      }
    }
  } catch (e) {
    $("connection").textContent = e.message;
  }
}
async function poll(jobId) {
  for (let n = 0; n < 90; n++) {
    await new Promise((r) => setTimeout(r, 2000));
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
    const origins = [
      ...new Set(
        snapshot.images
          .filter((i) => /^https?:/.test(i.url))
          .map((i) => new URL(i.url).origin + "/*"),
      ),
    ];
    if (origins.length && (await chrome.permissions.request({ origins }))) {
      prepared = undefined;
      status("Image host access granted. Capture when ready.");
    }
  } catch (e) {
    status(e.message);
  }
};
$("connect").onclick = async () => {
  try {
    const receiver = connection();
    if (
      receiver.startsWith("https:") &&
      !(await chrome.permissions.request({ origins: [receiver + "/*"] }))
    )
      throw Error("GG server access was not granted");
    const data = await request("/pair", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ name: "Browser extension" }),
    });
    $("token").value = data.token;
    await chrome.storage.local.set({ receiver, token: data.token });
    $("settings").open = false;
    $("connection").textContent = "Connected to GG.";
    await refresh();
  } catch (e) {
    $("connection").textContent = e.message;
  }
};
$("notifications").onchange = async () => {
  if (
    $("notifications").checked &&
    (await Notification.requestPermission()) !== "granted"
  )
    $("notifications").checked = false;
  await chrome.storage.local.set({ notifications: $("notifications").checked });
};
if (settings.token) await refresh();
