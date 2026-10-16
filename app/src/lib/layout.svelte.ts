// The sizes of Clarity's own panels, as the user dragged them. Remembered
// in this browser only; losing them just means the default sizes return.

const KEY = "clarity.layout";

type Sizes = {
  sidebar: number;
  properties: number;
  codePanel: number;
  output: number;
};

const DEFAULTS: Sizes = { sidebar: 300, properties: 280, codePanel: 640, output: 240 };

/** The smallest and largest size of each panel, in pixels. */
const LIMITS: Record<keyof Sizes, [number, number]> = {
  sidebar: [200, 600],
  properties: [220, 560],
  codePanel: [360, 1400],
  output: [100, 700],
};

function read(): Sizes {
  try {
    const stored = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Sizes>;
    return { ...DEFAULTS, ...stored };
  } catch {
    return { ...DEFAULTS };
  }
}

class Layout {
  sizes = $state<Sizes>(read());

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
