import { prepareCapture } from "./capture-media.js";
async function startCapture(tab, openPage = true) {
  try {
    if (!tab.id || !/^https?:/.test(tab.url ?? ""))
      throw Error("Open a normal web page before capturing.");
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["snapshot.js"],
    });
    const [{ result }] = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => globalThis.ggSnapshot(),
    });
    // Abandoned capture tabs should not fill Chrome's session storage quota.
    const entries = Object.entries(await chrome.storage.session.get(null))
      .filter(([key]) => key.startsWith("capture_"))
      .sort((a, b) =>
        a[1].snapshot.capturedAt.localeCompare(b[1].snapshot.capturedAt),
      );
    await chrome.storage.session.remove(
      entries.slice(0, Math.max(0, entries.length - 2)).map(([key]) => key),
    );
    const id = crypto.randomUUID();
    await chrome.storage.session.set({
      ["capture_" + id]: { snapshot: result, tabId: tab.id },
    });
    if (openPage)
      await chrome.tabs.create({
        url: chrome.runtime.getURL("capture.html") + "?id=" + id,
      });
    return { id };
  } catch (error) {
    if (!openPage) return { error: error.message };
    await chrome.tabs.create({
      url:
        chrome.runtime.getURL("capture.html") +
        "?error=" +
        encodeURIComponent(error.message),
    });
  }
}
globalThis.startCapture = startCapture;
const pending = new Map();
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (
    sender.id !== chrome.runtime.id ||
    !sender.url?.startsWith(chrome.runtime.getURL("capture.html"))
  )
    return;
  let task;
  if (message.type === "prepare-popup") {
    task = chrome.tabs
      .get(message.tabId)
      .then((tab) => startCapture(tab, false));
  } else if (message.type === "submit-popup") {
    task = pending.get(message.id);
    if (!task) {
      task = submitPopup(message);
      pending.set(message.id, task);
      task.finally(() => pending.delete(message.id)).catch(() => {});
    }
  } else return;
  task.then(respond, (error) => respond({ error: error.message }));
  return true;
});
async function submitPopup({ id, instructions }) {
  const key = "capture_" + id;
  const entry = (await chrome.storage.session.get(key))[key];
  if (!entry) throw Error("Capture expired. Open GG on the source page again.");
  const { receiver, token } = await chrome.storage.local.get([
    "receiver",
    "token",
  ]);
  if (!token) throw Error("Connect to GG in Settings first.");
  const url = new URL(receiver);
  if (
    (url.protocol !== "https:" &&
      !(url.protocol === "http:" && url.hostname === "127.0.0.1")) ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  )
    throw Error("Invalid GG server address");
  const prepared = await prepareCapture(
    entry.snapshot,
    entry.tabId,
    instructions,
  );
  const response = await fetch(url.origin + "/captures", {
    method: "POST",
    redirect: "error",
    headers: {
      authorization: "Bearer " + token,
      "content-type": "application/json",
      "x-gg-capture-id": id,
    },
    body: JSON.stringify(prepared),
    signal: AbortSignal.timeout(120000),
  });
  const result = await response.json();
  if (!response.ok) throw Error(result.error ?? "GG request failed");
  await chrome.storage.session.remove(key);
  return {
    id: result.id,
    missingImages: prepared.images.filter((image) => image.error).length,
  };
}
