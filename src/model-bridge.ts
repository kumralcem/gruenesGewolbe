import type { Context, AssistantMessage } from "@earendil-works/pi-ai";
export const emptyUsage = () => ({
  input: 0,
  output: 0,
  cacheRead: 0,
  cacheWrite: 0,
  totalTokens: 0,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
});
/** Compatibility only for deterministic, no-network fixture models. */
export async function fixtureCompletion(
  context: Context,
  model: string,
  mock: (body: any) => Promise<any>,
): Promise<AssistantMessage> {
  const messages: any[] = [];
  if (context.systemPrompt)
    messages.push({ role: "system", content: context.systemPrompt });
  for (const m of context.messages) {
    if (m.role === "user")
      messages.push({
        role: "user",
        content:
          typeof m.content === "string"
            ? m.content
            : m.content
                .filter((c) => c.type === "text")
                .map((c) => c.text)
                .join("\n"),
      });
    if (m.role === "assistant")
      messages.push({
        role: "assistant",
        content: m.content
          .filter((c) => c.type === "text")
          .map((c) => c.text)
          .join("\n"),
        tool_calls: m.content
          .filter((c) => c.type === "toolCall")
          .map((c) => ({
            id: c.id,
            type: "function",
            function: { name: c.name, arguments: JSON.stringify(c.arguments) },
          })),
      });
    if (m.role === "toolResult")
      messages.push({
        role: "tool",
        tool_call_id: m.toolCallId,
        content: m.content
          .filter((c) => c.type === "text")
          .map((c) => c.text)
          .join("\n"),
      });
  }
  const result = await mock({ model, messages });
  return {
    role: "assistant",
    provider: "gg",
    api: "openai-completions",
    model,
    timestamp: Date.now(),
    usage: emptyUsage(),
    stopReason: result.tool_calls?.length ? "toolUse" : "stop",
    content: [
      ...(result.content
        ? [{ type: "text" as const, text: result.content }]
        : []),
      ...(result.tool_calls ?? []).map((c: any) => ({
        type: "toolCall",
        id: c.id,
        name: c.function.name,
        arguments: JSON.parse(c.function.arguments),
      })),
    ],
  };
}
