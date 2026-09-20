/** Content-addressed provenance, never a filesystem path or fetchable URL. */
export const isLocalSource = (value: string): boolean =>
  /^gg-local:sha256:[a-f0-9]{64}$/.test(value);
