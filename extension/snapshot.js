// Runs in Chrome's isolated content-script world only after an explicit capture.
function ggSnapshot() {
  const selected = window.getSelection()?.toString().trim() ?? "";
  const url = location.href;
  const statusId = url.match(/\/(?:status|statuses)\/(\d+)/)?.[1];
  const mainPost = statusId
    ? Array.from(document.querySelectorAll("article")).find((a) =>
        Array.from(a.querySelectorAll("a[href]")).some((link) =>
          new URL(link.href, location.href).pathname.endsWith(
            "/status/" + statusId,
          ),
        ),
      )
    : null;
  const region =
    mainPost ??
    document.querySelector('article,main,[role="main"]') ??
    document.body;
  const visible = (element) =>
    element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true });
  const visibleText = (node) => {
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    const parts = [];
    while (walker.nextNode()) {
      const p = walker.currentNode.parentElement;
      if (
        !p ||
        p.closest(
          'script,style,form,input,textarea,select,[contenteditable],[hidden],[aria-hidden="true"]',
        )
      )
        continue;
      if (!visible(p)) continue;
      const text = walker.currentNode.textContent.trim();
      if (text) parts.push(text);
    }
    return parts.join("\n");
  };
  const text =
    selected ||
    (mainPost
      ? Array.from(mainPost.querySelectorAll('[data-testid="tweetText"]'))
          .slice(0, 1)
          .map((e) => e.innerText)
          .join("\n")
      : visibleText(region));
  const transcript = Array.from(
    document.querySelectorAll(
      "ytd-transcript-segment-renderer,yt-transcript-segment-view-model",
    ),
  )
    .map((e) => e.innerText)
    .join("\n");
  const clone = region.cloneNode(true);
  const originals = Array.from(region.querySelectorAll("*"));
  const copies = Array.from(clone.querySelectorAll("*"));
  for (let i = 0; i < originals.length; i++)
    if (!visible(originals[i])) copies[i].remove();
  clone
    .querySelectorAll(
      'script,style,iframe,object,embed,form,input,textarea,select,button,[contenteditable],[hidden],[aria-hidden="true"],noscript,template,nav,header,footer',
    )
    .forEach((e) => e.remove());
  for (const element of [clone, ...clone.querySelectorAll("*")])
    for (const attr of Array.from(element.attributes))
      if (
        !["href", "src", "alt", "title"].includes(attr.name) ||
        /^(?:javascript|data):/i.test(attr.value)
      )
        element.removeAttribute(attr.name);
  const images = Array.from(region.querySelectorAll("img"))
    .filter(
      (i) => i.naturalWidth >= 120 && i.naturalHeight >= 120 && visible(i),
    )
    .map((i) => ({
      url: i.currentSrc || i.src,
      alt: i.alt.slice(0, 300),
      width: i.naturalWidth,
      height: i.naturalHeight,
    }));
  const unique = Array.from(
    new Map(images.map((image) => [image.url, image])).values(),
  ).slice(0, 60);
  if (text.length > 400000 || transcript.length > 400000)
    throw Error(
      "This page is too long for one capture. Select the relevant text first.",
    );
  return {
    version: 1,
    url,
    title: document.title.slice(0, 1000),
    capturedAt: new Date().toISOString(),
    text,
    html: selected ? undefined : clone.outerHTML.slice(0, 1500000),
    transcript: transcript || undefined,
    images: unique,
    selection: Boolean(selected),
  };
}
globalThis.ggSnapshot = ggSnapshot;
