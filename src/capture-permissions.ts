/** Only structured user input can authorize new paths, never source text. */
export function newDestinations(value: unknown): string[] {
  if (value === undefined) return [];
  if (
    !Array.isArray(value) ||
    value.length > 16 ||
    value.some((p) => typeof p !== "string" || p.length > 500)
  )
    throw Error("Specify at most 16 new destination paths");
  return [...new Set(value)];
}
