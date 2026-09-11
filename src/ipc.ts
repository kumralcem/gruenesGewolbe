import net from "node:net";
import { randomUUID } from "node:crypto";
import { Duplex } from "node:stream";

// Multiplex TCP conversations over the single inherited socket. The host side
// connects only to its own gateway, never to a path supplied by the worker.
export class Channel {
  private sockets = new Map<string, net.Socket>();
  private pending = "";
  private transferred = 0;
  constructor(
    private pipe: Duplex,
    private destination?: string,
  ) {
    pipe.on("data", (chunk: Buffer) => {
      this.pending += chunk.toString();
      if (this.pending.length > 2_000_000) {
        this.close();
        return;
      }
      let at;
      while ((at = this.pending.indexOf("\n")) >= 0) {
        const line = this.pending.slice(0, at);
        this.pending = this.pending.slice(at + 1);
        try {
          this.receive(JSON.parse(line));
        } catch {
          this.close();
        }
      }
    });
    pipe.on("error", () => this.close());
    pipe.on("close", () => this.close());
  }
  attach(socket: net.Socket, id = randomUUID(), announce = true) {
    if (this.sockets.size >= 32) {
      socket.destroy();
      return;
    }
    this.sockets.set(id, socket);
    if (announce) this.send({ id, type: "open" });
    socket.on("data", (data: Buffer) =>
      this.send({ id, type: "data", data: data.toString("base64") }),
    );
    socket.on("end", () => this.send({ id, type: "end" }));
    socket.on("error", () => this.send({ id, type: "close" }));
    socket.on("close", () => {
      this.sockets.delete(id);
      this.send({ id, type: "close" });
    });
  }
  private send(value: unknown) {
    if (!this.pipe.destroyed) this.pipe.write(JSON.stringify(value) + "\n");
  }
  private receive(frame: any) {
    if (typeof frame.id !== "string" || frame.id.length > 60)
      throw Error("Invalid channel");
    if (frame.type === "open") {
      if (!this.destination || this.sockets.has(frame.id))
        throw Error("Invalid channel open");
      this.attach(net.connect(this.destination), frame.id, false);
      return;
    }
    const socket = this.sockets.get(frame.id);
    if (!socket) return;
    if (frame.type === "data") {
      if (typeof frame.data !== "string" || frame.data.length > 1_000_000)
        throw Error("Invalid channel data");
      const data = Buffer.from(frame.data, "base64");
      this.transferred += data.length;
      if (this.transferred > 512_000_000)
        throw Error("Channel byte budget exceeded");
      socket.write(data);
    } else if (frame.type === "end") socket.end();
    else if (frame.type === "close") socket.destroy();
  }
  close() {
    for (const socket of this.sockets.values()) socket.destroy();
    this.sockets.clear();
    this.pipe.destroy();
  }
}
