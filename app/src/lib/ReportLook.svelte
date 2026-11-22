<script lang="ts">
  // The look of one rule of a report's stylesheet (docs/ledger/reports-pdf.md):
  // its text's size, weight, slant, colour, background, font and alignment,
  // as the stylesheet's CSS writes them. Used for a part's class in its
  // properties and for any rule (body, h1, .title) in the Styles tab: one
  // look language, written back into the stylesheet.

  import { stylesheetRules } from "./reportBackend";

  let {
    css,
    selector,
    onset,
  }: {
    /** The stylesheet's text. */
    css: string;
    /** The rule it shows: `.title`, `body`, `h1`. */
    selector: string;
    /** Sets one declaration of the rule; an empty value takes it out. */
    onset: (property: string, value: string) => void;
  } = $props();

  let rule = $state<Record<string, string>>({});
  $effect(() => {
    const asked = css;
    void stylesheetRules(asked).then((rules) => {
      if (css === asked) rule = rules[selector] ?? {};
    });
  });
  const size = $derived((rule["font-size"] ?? "").replace(/pt$/, ""));
  const bold = $derived(rule["font-weight"] === "bold" || Number(rule["font-weight"]) >= 600);
  const italic = $derived(rule["font-style"] === "italic");
  /** A colour as the colour picker takes it: `#rrggbb`, else black. */
  const hex = (v: string | undefined, fallback: string) => (v && /^#[0-9a-fA-F]{6}$/.test(v) ? v : fallback);
  const FONTS = ["", "sans-serif", "serif", "monospace", "Inter", "Libertinus Serif", "New Computer Modern", "DejaVu Sans Mono"];
</script>

<div class="look">
  <label>Size (pt) <input type="number" min="4" max="96" step="0.5" value={size} placeholder="as body" onchange={(e) => onset("font-size", e.currentTarget.value ? `${e.currentTarget.value}pt` : "")} /></label>
  <div class="row">
    <label class="check"><input type="checkbox" checked={bold} onchange={(e) => onset("font-weight", e.currentTarget.checked ? "bold" : "")} /> Bold</label>
    <label class="check"><input type="checkbox" checked={italic} onchange={(e) => onset("font-style", e.currentTarget.checked ? "italic" : "")} /> Italic</label>
  </div>
  <label>Colour
    <span class="colour">
      <input type="color" value={hex(rule.color, "#1d2330")} onchange={(e) => onset("color", e.currentTarget.value)} />
      <input value={rule.color ?? ""} placeholder="as body" spellcheck="false" onchange={(e) => onset("color", e.currentTarget.value.trim())} />
    </span>
  </label>
  <label>Background
    <span class="colour">
      <input type="color" value={hex(rule["background-color"], "#ffffff")} onchange={(e) => onset("background-color", e.currentTarget.value)} />
      <input value={rule["background-color"] ?? ""} placeholder="none" spellcheck="false" onchange={(e) => onset("background-color", e.currentTarget.value.trim())} />
    </span>
  </label>
  <label>Font
    <select value={rule["font-family"] ?? ""} onchange={(e) => onset("font-family", e.currentTarget.value)}>
      {#each FONTS as f (f)}<option value={f}>{f || "as body"}</option>{/each}
    </select>
  </label>
  <label>Alignment
    <select value={rule["text-align"] ?? ""} onchange={(e) => onset("text-align", e.currentTarget.value)}>
      <option value="">as written</option>
      <option value="left">left</option>
      <option value="center">center</option>
      <option value="right">right</option>
      <option value="justify">justify</option>
    </select>
  </label>
</div>

<style>
  .look {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.9em;
  }
  .row {
    display: flex;
    gap: 12px;
  }
  .check {
    flex-direction: row;
    align-items: center;
    gap: 4px;
  }
  .colour {
    display: flex;
    gap: 4px;
  }
  .colour input[type="color"] {
    width: 36px;
    padding: 0;
  }
  .colour input:not([type="color"]) {
    flex: 1;
    min-width: 0;
  }
</style>
