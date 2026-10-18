// The pictures of records on the web renderer: an image field is null, a
// stored picture ({ stored: true }) fetched on its own, or a new one loaded
// from a file ({ data: base64 }) until the record is saved. The runtimes
// read and write the same (conformance/pictures.json).

export type PictureKind = { extension: string; type: string };

/** The kind of picture `bytes` are, told by their first bytes; null for anything else. */
export function kindOf(bytes: Uint8Array): PictureKind | null {
  const starts = (...prefix: number[]) => prefix.every((b, i) => bytes[i] === b);
  const ascii = (at: number, text: string) => [...text].every((c, i) => bytes[at + i] === c.charCodeAt(0));
  if (starts(0x89, 0x50, 0x4e, 0x47)) return { extension: "png", type: "image/png" };
  if (starts(0xff, 0xd8, 0xff)) return { extension: "jpg", type: "image/jpeg" };
  if (ascii(0, "GIF8")) return { extension: "gif", type: "image/gif" };
  if (ascii(0, "BM")) return { extension: "bmp", type: "image/bmp" };
  if (ascii(0, "RIFF") && ascii(8, "WEBP")) return { extension: "webp", type: "image/webp" };
  return null;
}

function toBase64(bytes: Uint8Array): string {
  let text = "";
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(text);
}

function fromBase64(data: string): Uint8Array {
  return Uint8Array.from(atob(data.slice(0, 24)), (c) => c.charCodeAt(0));
}

/** A chosen file as the new picture of a field; null when it is no picture. */
export function pictureFromFile(bytes: Uint8Array): { data: string } | null {
  return kindOf(bytes) ? { data: toBase64(bytes) } : null;
}

/**
 * What a field's picture shows as: a new one from its own bytes, a stored
 * one once `fetched` (its URL) has come, nothing without one.
 */
export function pictureUrl(value: unknown, fetched: string | null | undefined): string | null {
  if (value && typeof value === "object" && "data" in value && typeof value.data === "string") {
    const kind = kindOf(fromBase64(value.data));
    return kind ? `data:${kind.type};base64,${value.data}` : null;
  }
  if (value && typeof value === "object" && "stored" in value) return fetched ?? null;
  return null;
}

/** Whether a field holds a stored picture, to be fetched on its own. */
export function isStored(value: unknown): boolean {
  return !!value && typeof value === "object" && "stored" in value && !("data" in value);
}
