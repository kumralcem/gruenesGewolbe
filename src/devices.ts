import { createHash, randomBytes, randomUUID } from "node:crypto";
import { join } from "node:path";
import { fileLock, readJson, writeJson } from "./state.ts";
type Scope = "capture" | "manage";
interface Device {
  id: string;
  name: string;
  scope: Scope;
  hash: string;
  createdAt: string;
}
interface State {
  devices: Device[];
  codes: { hash: string; scope: Scope; expires: number }[];
}
const digest = (s: string) => createHash("sha256").update(s).digest("hex");
export class Devices {
  constructor(private dir: string) {}
  private async state<T>(fn: (s: State) => Promise<T> | T) {
    return fileLock(join(this.dir, "devices.lock"), async () => {
      const s = await readJson<State>(join(this.dir, "devices.json"), {
        devices: [],
        codes: [],
      });
      s.codes = s.codes.filter((c) => c.expires > Date.now());
      const result = await fn(s);
      await writeJson(join(this.dir, "devices.json"), s);
      return result;
    });
  }
  async pairing(scope: Scope = "capture") {
    if (!["capture", "manage"].includes(scope))
      throw Error("Invalid device scope");
    const code = randomBytes(24).toString("hex");
    await this.state((s) => {
      s.codes.push({ hash: digest(code), scope, expires: Date.now() + 600000 });
      s.codes = s.codes.slice(-20);
    });
    return code;
  }
  async exchange(code: string, name: string) {
    return this.state((s) => {
      const at = s.codes.findIndex((c) => c.hash === digest(code));
      if (at < 0) throw Error("Invalid or expired pairing code");
      if (typeof name !== "string" || !name.trim() || name.length > 100)
        throw Error("Invalid device name");
      const [pair] = s.codes.splice(at, 1);
      const token = randomBytes(32).toString("hex"),
        id = randomUUID();
      s.devices.push({
        id,
        name,
        scope: pair.scope,
        hash: digest(token),
        createdAt: new Date().toISOString(),
      });
      return { id, token, scope: pair.scope };
    });
  }
  async authenticate(token: string) {
    const s = await readJson<State>(join(this.dir, "devices.json"), {
      devices: [],
      codes: [],
    });
    const found = s.devices.find((d) => d.hash === digest(token));
    if (!found) return undefined;
    const { hash, ...device } = found;
    return device;
  }
  async list() {
    return this.state((s) => s.devices.map(({ hash, ...d }) => d));
  }
  async revoke(id: string) {
    return this.state((s) => {
      if (!s.devices.some((d) => d.id === id)) throw Error("Unknown device");
      s.devices = s.devices.filter((d) => d.id !== id);
    });
  }
}
