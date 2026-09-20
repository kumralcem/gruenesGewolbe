import { prepareCapture } from "./capture-media.js";
const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
const popup = params.has("popup");
if (popup) document.body.classList.add("popup");
$("version").textContent = "GG " + chrome.runtime.getManifest().version;
$("dashboard").onclick = () =>
  chrome.tabs.create({ url: chrome.runtime.getURL("capture.html") });
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
if (popup) {
  try {
    const [tab] = await chrome.tabs.query({
      active: true,
      currentWindow: true,
    });
    const result = await chrome.runtime.sendMessage({
      type: "prepare-popup",
      tabId: tab?.id,
    });
    if (result.error) throw Error(result.error);
    id = result.id;
  } catch (error) {
    params.set("error", error.message);
  }
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
async function send() {
  $("capture").disabled = true;
  try {
    status("Collecting the page and images…");
    if (popup) {
      const result = await chrome.runtime.sendMessage({
        type: "submit-popup",
        id,
        instructions: $("focus").value,
      });
      if (result.error) throw Error(result.error);
      status(
        `Capture received. You can close this popup.${result.missingImages ? ` ${result.missingImages} image(s) unavailable; available content was sent.` : ""}`,
      );
      $("focus").disabled = true;
      $("allow").disabled = true;
      return;
    }
    if (!prepared) {
      prepared = await prepareCapture(snapshot, tabId, $("focus").value);
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
if (popup && !settings.token) {
  $("capture").disabled = true;
  status("Connect to GG first using Settings & recent captures below.");
}
if (settings.token && !popup) await refresh();
