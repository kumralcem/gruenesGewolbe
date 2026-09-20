/** Plain UTF-8 formats only: no document rendering or execution. */
export const textExtensions = new Set([
  "md",
  "markdown",
  "txt",
  "rst",
  "csv",
  "tsv",
  "json",
  "yaml",
  "yml",
  "toml",
  "log",
]);
export const maxTextBytes = 64_000;
export function validateImportedText(text: string) {
  if (
    typeof text !== "string" ||
    !text.trim() ||
    Buffer.byteLength(text, "utf8") > maxTextBytes ||
    /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(text) ||
    Buffer.from(text, "utf8").toString("utf8") !== text
  )
    throw Error(
      "Text import requires nonempty UTF-8 text, at most 64 KB, without binary control characters",
    );
}
export function originalTextFile(extension: string) {
  if (!textExtensions.has(extension)) throw Error("Unsupported text format");
  return `original.${extension}`;
}
