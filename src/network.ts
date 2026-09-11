import { lookup } from "node:dns/promises";
import { BlockList, isIP } from "node:net";
import http from "node:http";
import https from "node:https";

const blocked = new BlockList();
for (const [base, bits] of [
  ["0.0.0.0", 8],
  ["10.0.0.0", 8],
  ["100.64.0.0", 10],
  ["127.0.0.0", 8],
  ["169.254.0.0", 16],
  ["172.16.0.0", 12],
  ["192.0.0.0", 24],
  ["192.0.2.0", 24],
  ["192.168.0.0", 16],
  ["198.18.0.0", 15],
  ["198.51.100.0", 24],
  ["203.0.113.0", 24],
  ["224.0.0.0", 4],
  ["240.0.0.0", 4],
] as const)
  blocked.addSubnet(base, bits, "ipv4");
const global6 = new BlockList();
global6.addSubnet("2000::", 3, "ipv6");
blocked.addSubnet("2001::", 32, "ipv6");
blocked.addSubnet("2002::", 16, "ipv6");
blocked.addSubnet("2001:db8::", 32, "ipv6");
export function publicIp(ip: string) {
  const family = isIP(ip);
  return family === 4
    ? !blocked.check(ip, "ipv4")
    : family === 6 && global6.check(ip, "ipv6") && !blocked.check(ip, "ipv6");
}
export async function publicTarget(raw: string) {
  const url = new URL(raw);
  if (
    !["http:", "https:"].includes(url.protocol) ||
    url.username ||
    url.password ||
    (url.port && !["80", "443"].includes(url.port))
  )
    throw Error("Only public HTTP(S) on standard ports is allowed");
  const host = url.hostname.replace(/^\[|\]$/g, "");
  const addresses = isIP(host)
    ? [{ address: host, family: isIP(host) }]
    : await lookup(host, { all: true });
  if (!addresses.length || addresses.some((a) => !publicIp(a.address)))
    throw Error("Private/reserved network target denied");
  return {
    url,
    address: addresses.find((a) => a.family === 4) ?? addresses[0],
  };
}
export async function fetchPublic(
  raw: string,
  signal: AbortSignal,
  limit = 64_000_000,
  redirects = 0,
): Promise<{ bytes: Buffer; type: string; url: string }> {
  if (redirects > 5) throw Error("Too many redirects");
  const { url, address } = await publicTarget(raw);
  return new Promise((resolve, reject) => {
    const request = (url.protocol === "https:" ? https : http).get(
      url,
      {
        signal,
        family: address.family,
        lookup: (_host, opts, cb) =>
          opts.all
            ? cb(null, [address])
            : cb(null, address.address, address.family),
        headers: {
          "User-Agent": "GG-Prototype/0.1 (personal archival research)",
        },
      },
      (response) => {
        if (
          [301, 302, 303, 307, 308].includes(response.statusCode ?? 0) &&
          response.headers.location
        ) {
          response.resume();
          fetchPublic(
            new URL(response.headers.location, url).href,
            signal,
            limit,
            redirects + 1,
          ).then(resolve, reject);
          return;
        }
        if ((response.statusCode ?? 500) >= 400) {
          response.resume();
          reject(Error(`Source returned HTTP ${response.statusCode}`));
          return;
        }
        let size = 0;
        const chunks: Buffer[] = [];
        response.on("data", (b: Buffer) => {
          size += b.length;
          if (size > limit)
            response.destroy(Error("Download size limit exceeded"));
          else chunks.push(b);
        });
        response.on("error", reject);
        response.on("end", () =>
          resolve({
            bytes: Buffer.concat(chunks),
            type: response.headers["content-type"] ?? "",
            url: url.href,
          }),
        );
      },
    );
    request.setTimeout(20000, () => request.destroy(Error("Source timeout")));
    request.on("error", reject);
  });
}
