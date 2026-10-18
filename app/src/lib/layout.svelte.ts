// The sizes of Clarity's own panels, as the user dragged them. Remembered
// in this browser only; losing them just means the default sizes return.

const KEY = "clarity.layout";

type Sizes = {
  sidebar: number;
  properties: number;
  codePanel: number;
  output: number;
  agent: number;
};

const DEFAULTS: Sizes = { sidebar: 300, properties: 280, codePanel: 640, output: 240, agent: 380 };

/** The smallest and largest size of each panel, in pixels. */
const LIMITS: Record<keyof Sizes, [number, number]> = {
  sidebar: [200, 600],
  properties: [220, 560],
  codePanel: [360, 1400],
  output: [100, 700],
  agent: [280, 900],
};

function read(): Sizes {
  try {
    const stored = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Sizes>;
    return { ...DEFAULTS, ...stored };
  } catch {
    return { ...DEFAULTS };
  }
}

const FOLDED_KEY = "clarity.folded";

/** The panels and sections that can be folded away. */
type Foldable = "properties" | "window" | "picked";

function readFolded(): Partial<Record<Foldable, boolean>> {
  try {
    return JSON.parse(localStorage.getItem(FOLDED_KEY) ?? "{}") as Partial<Record<Foldable, boolean>>;
  } catch {
    return {};
  }
}

class Layout {
  sizes = $state<Sizes>(read());
  folded = $state<Partial<Record<Foldable, boolean>>>(readFolded());

  /** Folds a panel or section away, or opens it again, and remembers it. */
  toggle(which: Foldable) {
    this.folded[which] = !this.folded[which];
    try {
      localStorage.setItem(FOLDED_KEY, JSON.stringify(this.folded));
    } catch {
      // Remembering is a convenience only.
    }
  }

  /** Changes a size by `delta` pixels, within its limits, and remembers it. */
  resize(which: keyof Sizes, delta: number) {
    const [min, max] = LIMITS[which];
    this.sizes[which] = Math.round(Math.min(max, Math.max(min, this.sizes[which] + delta)));
    try {
      localStorage.setItem(KEY, JSON.stringify(this.sizes));
    } catch {
      // Remembering sizes is a convenience only.
    }
  }
}

export const layout = new Layout();
