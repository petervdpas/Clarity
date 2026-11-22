// How Clarity writes a name, as clarity_spec::names does (conformance/names.json).

/** Whether `name` is camelCase: a small letter, then letters and digits (clarity_spec::names::is_camel_case). */
export function isCamelCase(name: string): boolean {
  return /^[a-z][A-Za-z0-9]*$/.test(name);
}

/** `words` as camelCase: `New game` and `new_game` are `newGame` (clarity_spec::names::camel_case). */
export function camelCase(words: string): string {
  const parts = words.match(/[A-Za-z0-9]+/g) ?? [];
  const joined = parts.map((w, i) => (i === 0 ? w[0].toLowerCase() + w.slice(1) : w[0].toUpperCase() + w.slice(1))).join("");
  return /^[0-9]/.test(joined) ? "n" + joined : joined;
}
