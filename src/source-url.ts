/** Match only known aliases of the same browser-captured resource. */
export function sameCapturedSource(left: string, right: string): boolean {
  try {
    const key = (value: string) => {
      const url = new URL(value);
      if (!["http:", "https:"].includes(url.protocol)) return value;
      const host = url.hostname.toLowerCase();
      if (
        [
          "x.com",
          "www.x.com",
          "twitter.com",
          "www.twitter.com",
          "mobile.twitter.com",
        ].includes(host)
      ) {
        const id = url.pathname.match(/^\/[^/]+\/status\/(\d+)\/?$/)?.[1];
        if (id) return "x-status:" + id;
      }
      if (
        [
          "youtube.com",
          "www.youtube.com",
          "m.youtube.com",
          "youtu.be",
        ].includes(host)
      ) {
        const id =
          host === "youtu.be"
            ? url.pathname.slice(1)
            : url.pathname === "/watch"
              ? url.searchParams.get("v")
              : null;
        if (id && /^[a-zA-Z0-9_-]{11}$/.test(id)) return "youtube-video:" + id;
      }
      url.hash = "";
      return url.href;
    };
    return key(left) === key(right);
  } catch {
    return false;
  }
}
