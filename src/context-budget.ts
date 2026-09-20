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

export function estimateInput(context: Context) {
  let imageTokens = 0;
  const text = JSON.stringify(context, (key, value) => {
    if (value && typeof value === "object" && value.type === "image") {
      imageTokens += 8192;
      return { type: "image" };
    }
    return value;
  });
  // UTF-8 bytes conservatively bound ordinary text tokens. Image accounting is
  // an explicit estimate; reported provider usage replaces reservations.
  return Buffer.byteLength(text) + imageTokens;
}

/** Close local-artwork research before there is no room left to save. Retain all
 * evidence and tool history; only narrow the next request's available tools. */
export function finalizationContext(context: Context): Context {
  return {
    ...context,
    tools: context.tools?.filter((tool) =>
      [
        "capture",
        "capture_policy",
        "create_destination",
        "plan_capture",
        "skip_capture",
      ].includes(tool.name),
    ),
    systemPrompt:
      (context.systemPrompt ?? "") +
      "\nThe research budget is now closed. Save the original image now using existing asset IDs and the evidence already available. Mark unresolved attribution uncertain. If needed, read the destination policy first. Do not request more research or repeat inspection.",
  };
}
