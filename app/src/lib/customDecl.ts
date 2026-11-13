// The lines of a custom part (`part Arena`) and a library (`library Words`)
// the IDE's forms and New dialogs write, one per thing, with the keys that
// name them (the spec's decl_lines): `library`, `binds`, `size`, `preview`,
// `property speed`, `tells scored`, `hears restart`, `source`,
// `function spell`.

import type { Backend } from "@clarity/ui/generated/Backend.ts";
import type { LibrarySource } from "@clarity/ui/generated/LibrarySource.ts";
import type { PartLibrary } from "@clarity/ui/generated/PartLibrary.ts";

/** One line of a declaration set by its key; a null line takes it away. */
export type DeclLine = { key: string; line: string | null };

export type PartKind = PartLibrary["kind"];
export type LibraryKind = LibrarySource["kind"];

/** The library kinds of a custom part, and the stacks each is written for. */
export const PART_KINDS: { kind: PartKind; label: string; backends: Backend[] }[] = [
  { kind: "native", label: "C or C++ in a folder, built with CMake", backends: ["c", "cpp"] },
  { kind: "crate", label: "a Rust crate in a folder", backends: ["rust"] },
  { kind: "go", label: "a Go package in a folder", backends: ["go"] },
  { kind: "nuget", label: "a .NET control from a NuGet package", backends: ["csharp"] },
  { kind: "dll", label: "a .NET control from an assembly", backends: ["csharp"] },
];

/** The package kinds of a library, and the stacks each is written for. */
export const LIBRARY_KINDS: { kind: LibraryKind; label: string; backends: Backend[] }[] = [
  { kind: "cmake", label: "a package CMake finds (C, C++)", backends: ["c", "cpp"] },
  { kind: "crate", label: "a Rust crate", backends: ["rust"] },
  { kind: "go", label: "a Go module", backends: ["go"] },
  { kind: "nuget", label: "a NuGet package (C#)", backends: ["csharp"] },
  { kind: "dll", label: "a .NET assembly in the project (C#)", backends: ["csharp"] },
];

/** The kinds a project of `backend` writes; every kind when it is not known. */
export function kindsFor<K>(kinds: { kind: K; label: string; backends: Backend[] }[], backend: Backend | null) {
  return backend ? kinds.filter((k) => k.backends.includes(backend)) : kinds;
}

/** Whether `name` is a part's or library's name: a capital, then letters and digits. */
export function isDeclName(name: string): boolean {
  return /^[A-Z][A-Za-z0-9]*$/.test(name);
}

/** A custom part's library to start from, in its own folder. */
export function startPartLibrary(kind: PartKind, name: string): PartLibrary {
  switch (kind) {
    case "native":
    case "crate":
      return { kind, folder: `parts/${name}` };
    case "go":
      return { kind, folder: `parts/${name.toLowerCase()}` };
    case "nuget":
      return { kind, package: "", version: "" };
    case "dll":
      return { kind, file: `libs/${name}.dll` };
  }
}

/** A library's package to start from. */
export function startLibrarySource(kind: LibraryKind, name: string): LibrarySource {
  switch (kind) {
    case "cmake":
      return { kind, package: name };
    case "crate":
      return { kind, name: name.toLowerCase(), version: "" };
    case "go":
      return { kind, module: "", version: "" };
    case "nuget":
      return { kind, package: "", version: "" };
    case "dll":
      return { kind, file: `libs/${name}.dll` };
  }
}

const q = (text: string) => JSON.stringify(text.trim());

/** `crate "parts/Arena"`, `nuget "Markdig" 0.40.0`. */
export function partLibraryLine(library: PartLibrary): string {
  switch (library.kind) {
    case "native":
    case "crate":
    case "go":
      return `${library.kind} ${q(library.folder)}`;
    case "nuget":
      return `nuget ${q(library.package)} ${library.version.trim()}`;
    case "dll":
      return `dll ${q(library.file)}`;
  }
}

/** `nuget "Humanizer.Core" 2.14.1`, `crate "humanize" "0.1"`, `cmake "SFML"`. */
export function librarySourceLine(source: LibrarySource): string {
  switch (source.kind) {
    case "nuget":
      return `nuget ${q(source.package)} ${source.version.trim()}`;
    case "dll":
      return `dll ${q(source.file)}`;
    case "crate":
      return `crate ${q(source.name)} ${q(source.version)}`;
    case "go":
      return `go ${q(source.module)} ${source.version.trim()}`;
    case "cmake":
      return `cmake ${q(source.package)}`;
  }
}

/** What is left to fill in before a package line reads: a nuget's version, a crate's name. */
export function missingOf(source: PartLibrary | LibrarySource): string | null {
  const blank = (t: string) => t.trim() === "";
  switch (source.kind) {
    case "native":
    case "crate":
      if ("folder" in source) return blank(source.folder) ? "the folder" : null;
      return blank(source.name) ? "the crate" : blank(source.version) ? "the version" : null;
    case "go":
      if ("folder" in source) return blank(source.folder) ? "the folder" : null;
      return blank(source.module) ? "the module" : blank(source.version) ? "the version" : null;
    case "nuget":
      return blank(source.package) ? "the package" : blank(source.version) ? "the version" : null;
    case "dll":
      return blank(source.file) ? "the assembly" : null;
    case "cmake":
      return blank(source.package) ? "the package" : null;
  }
}

/** A new custom part's declaration: its library and the size the Parts palette drops it with. */
export function newPartText(name: string, library: PartLibrary): string {
  return `part ${name}\n    ${partLibraryLine(library)}\n    size 6 x 4\n`;
}

/** A new library's declaration: its package; its functions are added in its view. */
export function newLibraryText(name: string, source: LibrarySource): string {
  return `library ${name}\n    ${librarySourceLine(source)}\n`;
}

/** `speed: integer, min 1, max 10, default 3`; blank values are left out. */
export function propertyLine(name: string, type: string, values: { min?: string; max?: string; default?: string }): DeclLine {
  const extra = (["min", "max", "default"] as const).filter((w) => values[w]?.trim()).map((w) => `, ${w} ${values[w]!.trim()}`);
  return { key: `property ${name}`, line: `${name}: ${type.trim()}${extra.join("")}` };
}

/** `tells scored(points: integer)`, `hears restart()`. */
export function messageLine(word: "tells" | "hears", name: string, params: string): DeclLine {
  return { key: `${word} ${name}`, line: `${word} ${name}(${params.trim()})` };
}

/** `function spell(n: integer) -> text`; without a result type it returns nothing. */
export function functionLine(name: string, params: string, returns: string): DeclLine {
  const result = returns.trim() ? ` -> ${returns.trim()}` : "";
  return { key: `function ${name}`, line: `function ${name}(${params.trim()})${result}` };
}

/** The text between a line's first `(` and its last `)`: the values a tell or function takes. */
export function paramsOf(line: string): string {
  const open = line.indexOf("(");
  const close = line.lastIndexOf(")");
  return open >= 0 && close > open ? line.slice(open + 1, close).trim() : "";
}

/** The text of the line at `span` in a declaration's text that starts at `item`. */
export function sliceOf(declText: string, item: number, span: { start: number; end: number }): string {
  return declText.slice(span.start - item, span.end - item).trim();
}

/** The types a property, a value or a result can have, then the project's enums. */
export function typeChoices(enums: string[]): string[] {
  return ["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", ...enums];
}
