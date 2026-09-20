export type Attribution = Partial<
  Record<
    "title" | "creator" | "year",
    {
      value: string;
      status: "filename" | "uncertain" | "source-supported";
      sourceUrl?: string;
      quote?: string;
    }
  >
>;
export function validateAttribution(
  value: unknown,
): asserts value is Attribution {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw Error("Invalid attribution");
  for (const [key, field] of Object.entries(value)) {
    if (
      !["title", "creator", "year"].includes(key) ||
      !field ||
      typeof field.value !== "string" ||
      !field.value.trim() ||
      field.value.length > 300 ||
      !["filename", "uncertain", "source-supported"].includes(field.status)
    )
      throw Error("Invalid attribution field");
    if (field.status === "source-supported") {
      if (
        typeof field.sourceUrl !== "string" ||
        field.sourceUrl.length > 4000 ||
        typeof field.quote !== "string" ||
        !field.quote.trim() ||
        field.quote.length > 1200
      )
        throw Error(
          "Source-supported attribution needs a source and supporting excerpt",
        );
      const url = new URL(field.sourceUrl);
      if (
        !["http:", "https:"].includes(url.protocol) ||
        url.username ||
        url.password
      )
        throw Error("Invalid attribution source");
    }
  }
}
export const evidenceText = (text: string) =>
  text
    .replace(/<[^>]*>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, " ")
    .trim();
/** Filename labels remain useful, but do not become verified facts. */
export function normalizeLocalAttribution(
  draft: {
    title: string;
    creator?: string;
    year?: string;
    attribution?: Attribution;
  },
  evidence?: Map<string, string>,
) {
  const attribution: Attribution = draft.attribution ?? {
    title: { value: draft.title, status: "filename" },
    ...(draft.creator
      ? { creator: { value: draft.creator, status: "filename" as const } }
      : {}),
    ...(draft.year
      ? { year: { value: draft.year, status: "filename" as const } }
      : {}),
  };
  validateAttribution(attribution);
  attribution.title ??= { value: draft.title, status: "uncertain" };
  for (const field of Object.values(attribution)) {
    if (
      field?.status === "source-supported" &&
      !evidenceText(field.quote!)
        .toLocaleLowerCase()
        .includes(evidenceText(field.value).toLocaleLowerCase())
    )
      throw Error("Attribution excerpt must contain the claimed value");
    if (
      field?.status === "source-supported" &&
      (!evidence?.get(field.sourceUrl!) ||
        !evidenceText(evidence.get(field.sourceUrl!)!).includes(
          evidenceText(field.quote!),
        ))
    )
      throw Error(
        "Attribution excerpt must come from a source fetched with fetch_text during this job",
      );
  }
  draft.attribution = attribution;
  draft.creator =
    attribution.creator?.status === "source-supported"
      ? attribution.creator.value
      : undefined;
  draft.year =
    attribution.year?.status === "source-supported"
      ? attribution.year.value
      : undefined;
  draft.title = attribution.title.value;
}
