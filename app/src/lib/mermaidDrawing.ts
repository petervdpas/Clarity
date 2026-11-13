// Mermaid, loaded once when something first draws with it, set to the IDE
// theme's colors: the diagram designer and the manual's diagrams use it.
import { themeVariables } from "./diagramCanvas";

type Mermaid = (typeof import("mermaid"))["default"];
let loading: Promise<Mermaid> | null = null;

/** Mermaid, initialized with the colors and font the IDE shows now. */
export async function themedMermaid(): Promise<Mermaid> {
  loading ??= import("mermaid").then((m) => m.default);
  const m = await loading;
  const css = getComputedStyle(document.documentElement);
  m.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: "base",
    // The font the page shows, so boxes are measured in the font they are drawn in.
    themeVariables: { ...themeVariables((n) => css.getPropertyValue(n)), fontFamily: getComputedStyle(document.body).fontFamily },
  });
  return m;
}
