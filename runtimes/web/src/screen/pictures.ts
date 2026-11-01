// Where the renderer finds a picture of the project's resources folder. A
// generated application carries the pictures next to its renderer; the
// designer asks Clarity for them, since the page cannot read the project
// folder, and shows each one once it has come.

/** Gives a picture by name as a URL the page can show. */
export type PictureLoader = (name: string) => Promise<string>;

/**
 * A picture next to the renderer, as the web stacks ship it; the C and C++
 * shells, whose page has no folder, carry them in the page itself.
 */
export function bundled(name: string): string {
  const carried = (globalThis as { clarityResources?: Record<string, string> }).clarityResources?.[name];
  return carried ?? `resources/${encodeURIComponent(name)}`;
}

export class Pictures {
  #load: PictureLoader | null;
  #changed: () => void;
  /** The URL of each picture loaded, or null for one that could not be. */
  #urls = new Map<string, string | null>();
  #asked = new Set<string>();

  constructor(load: PictureLoader | null, changed: () => void) {
    this.#load = load;
    this.#changed = changed;
  }

  /** The picture's URL; undefined while it loads, null when it is missing. */
  url(name: string): string | null | undefined {
    if (!this.#load) return bundled(name);
    if (this.#urls.has(name)) return this.#urls.get(name);
    if (!this.#asked.has(name)) {
      this.#asked.add(name);
      this.#load(name).then(
        (url) => this.#done(name, url),
        () => this.#done(name, null),
      );
    }
    return undefined;
  }

  #done(name: string, url: string | null) {
    this.#urls.set(name, url);
    this.#changed();
  }
}
