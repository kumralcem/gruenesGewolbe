import { test } from "node:test";
import assert from "node:assert/strict";
import { publicIp, publicTarget } from "../src/network.ts";
test("network guard rejects local, private, mapped IPv6 and metadata destinations", async () => {
  for (const ip of [
    "127.0.0.1",
    "10.1.2.3",
    "172.20.1.2",
    "192.168.1.1",
    "169.254.169.254",
    "100.64.0.1",
    "::1",
    "::ffff:127.0.0.1",
    "fc00::1",
    "fe80::1",
    "0.0.0.0",
  ])
    assert.equal(publicIp(ip), false, ip);
  assert.equal(publicIp("1.1.1.1"), true);
  assert.equal(publicIp("2606:4700:4700::1111"), true);
  for (const url of [
    "http://127.1/",
    "http://2130706433/",
    "file:///etc/passwd",
    "https://user:pass@example.com",
    "http://1.1.1.1:8080",
  ])
    await assert.rejects(publicTarget(url));
});
