import { createHash } from "node:crypto";
import type { Context } from "@earendil-works/pi-ai";

/** Repeated previews add no evidence. Keep the first copy, with a reference at
 * later occurrences. Source text and every tool call/result remain intact. */
export function deduplicateContextImages(context: Context): Context {
  const seen = new Set<string>();
  return {
    ...context,
    messages: context.messages.map((message) => {
      if (typeof message.content === "string") return message;
      return {
        ...message,
        content: message.content.map((part) => {
          if (part.type !== "image") return part;
          const hash = createHash("sha256").update(part.data).digest("hex");
          if (!seen.has(hash)) {
            seen.add(hash);
            return part;
          }
          return {
            type: "text" as const,
            text: "[Same image as the earlier preview; original bytes remain available through its asset ID.]",
          };
        }),
      };
    }),
  } as Context;
}

/** Public pages may contain enormous data URLs or tracking links. Keep original
 * source separately; the model receives bounded useful navigation candidates. */
export function boundedCandidates<T extends { url: string }>(
  values: T[],
  budget = 5000,
): T[] {
  const result: T[] = [];
  for (const value of values) {
    if (!/^https?:\/\//.test(value.url) || value.url.length > 2000) continue;
    const bytes = Buffer.byteLength(JSON.stringify(value));
    if (bytes > budget) continue;
    result.push(value);
    budget -= bytes;
    if (result.length === 20) break;
  }
  return result;
}
