async function startCapture(tab) {
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
    await chrome.tabs.create({
      url: chrome.runtime.getURL("capture.html") + "?id=" + id,
    });
  } catch (error) {
    await chrome.tabs.create({
      url:
        chrome.runtime.getURL("capture.html") +
        "?error=" +
        encodeURIComponent(error.message),
    });
  }
}
chrome.action.onClicked.addListener(startCapture);
chrome.commands.onCommand.addListener(async (command) => {
  if (command === "capture-page") {
    const [tab] = await chrome.tabs.query({
      active: true,
      currentWindow: true,
    });
    if (tab) await startCapture(tab);
  }
});
