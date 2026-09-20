import sharp from "sharp";
import { mkdir } from "node:fs/promises";
const root = new URL("../", import.meta.url);
await mkdir(new URL("extension/icons/", root), { recursive: true });
for (const size of [16, 32, 48, 128]) {
  await sharp(new URL("assets/brand/logo.svg", root).pathname)
    .resize(size, size)
    .png()
    .toFile(new URL(`extension/icons/gg-${size}.png`, root).pathname);
}
