// The resources a part shows by its key: each language's language file
// points the key at a file of the resources folder
// (`gameRules resource "rules-nl.md"`). A generated application has these
// resolved into its texts already; the designer looks them up here.

/** The file key `caption` (`@gameRules`) points at in language `picked`, else in the default language. */
export function keyResource(keys: Record<string, Record<string, string>> | undefined, caption: string, picked: string, fallback: string): string | undefined {
  if (!caption.startsWith("@")) return undefined;
  const key = caption.slice(1);
  return keys?.[picked]?.[key] ?? keys?.[fallback]?.[key];
}

export type ResourceKind = "markdown" | "picture" | "file";

const PICTURES = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/** What a resource is, by its name: Markdown, a picture every stack shows, or another file. */
export function resourceKind(name: string): ResourceKind {
  const extension = name.split(".").at(-1)?.toLowerCase() ?? "";
  if (extension === "md") return "markdown";
  return PICTURES.includes(extension) ? "picture" : "file";
}

/** The files a part or a key of `kind` can point at, by name. */
export function resourceChoices(files: { name: string }[] | undefined, kind: Exclude<ResourceKind, "file">): string[] {
  return (files ?? []).map((f) => f.name).filter((name) => resourceKind(name) === kind);
}

/** The keys that point at resource `name`, with the language: `@gameRules in nl`. */
export function resourceUses(keys: Record<string, Record<string, string>> | undefined, name: string): string[] {
  return Object.entries(keys ?? {}).flatMap(([language, pointed]) =>
    Object.entries(pointed)
      .filter(([, file]) => file === name)
      .map(([key]) => `@${key} in ${language}`),
  );
}
