<script lang="ts">
  // A theme's values as a form: every value the spec says a theme can set
  // (its catalog), each shown with the theme's own value or, greyed, the one
  // it keeps from Clarity's own theme. A change writes one line of the
  // theme's declaration; Reset takes the line away.

  import type { Look } from "@clarity/ui/generated/Look.ts";
  import type { ThemeCatalog } from "@clarity/ui/generated/ThemeCatalog.ts";
  import type { ThemeDecl } from "@clarity/ui/generated/ThemeDecl.ts";
  import {
    EMPTY_LOOK,
    colorLine,
    colorNames,
    fontLine,
    isHex,
    longHex,
    partLine,
    partLook,
    radiusLine,
    roleValue,
    settingLine,
    sizeLine,
    statesOf,
    styleLine,
    titleLine,
    type ThemeLine,
  } from "./themeLines.ts";

  let {
    theme,
    base,
    catalog,
    readonly = false,
    onset,
  }: {
    theme: ThemeDecl;
    /** Clarity's own theme: what the theme keeps where it sets nothing. */
    base: ThemeDecl;
    catalog: ThemeCatalog;
    /** Clarity's own theme is shown, not changed. */
    readonly?: boolean;
    /** Writes one line; throws with the reason when it is refused. */
    onset: (line: ThemeLine) => Promise<void>;
  } = $props();

  let error = $state<string | null>(null);
  let newColor = $state("");
  let newStyle = $state("");

  async function set(line: ThemeLine) {
    error = null;
    try {
      await onset(line);
    } catch (err) {
      error = String(err);
    }
  }

  const own = (name: string) => theme.colors.find((c) => c.name.name === name) ?? null;
  const inherited = (name: string) => base.colors.find((c) => c.name.name === name) ?? null;
  const extraColors = $derived(theme.colors.map((c) => c.name.name).filter((n) => !catalog.roles.includes(n)));
  const names = $derived(colorNames(catalog.roles, theme));

  /** Sets one mode of a color; a color the theme did not set keeps Clarity's other mode. */
  function setColor(name: string, mode: "light" | "dark", value: string) {
    const current = own(name) ?? inherited(name);
    const light = mode === "light" ? value : (current?.light ?? value);
    const dark = mode === "dark" ? value : (current?.dark ?? null);
    set(colorLine(name, light, dark));
  }

  function number(text: string): number | null {
    const n = Number.parseInt(text, 10);
    return Number.isFinite(n) && n >= 0 ? n : null;
  }

  /** The look with one value changed; empty text takes the value away. */
  function changed(look: Look | null, field: keyof Look, text: string): Look {
    const next: Look = { ...(look ?? EMPTY_LOOK) };
    if (field === "radius" || field === "size") next[field] = text === "" ? null : number(text);
    else if (field === "weight") next.weight = text === "bold" || text === "normal" ? text : null;
    else if (field === "align") next.align = text === "left" || text === "center" || text === "right" ? text : null;
    else next[field] = text === "" ? null : text;
    return next;
  }

  const FONT_CHOICES = [
    "system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
    "Inter, system-ui, sans-serif",
    "Georgia, serif",
    "Times New Roman, serif",
    "Verdana, sans-serif",
    "ui-monospace, Cascadia Code, Consolas, Menlo, monospace",
  ];
</script>

{#snippet colorPick(value: string | null, kept: string | null, none: boolean, onpick: (v: string) => void)}
  <span class="pick">
    <select value={value ?? ""} disabled={readonly} onchange={(e) => onpick(e.currentTarget.value)}>
      <option value="">{kept ? `as Clarity's: ${kept}` : "not set"}</option>
      {#if none}<option value="none">none</option>{/if}
      {#each names as n (n)}<option value={n}>{n}</option>{/each}
      {#if value && isHex(value)}<option value={value}>{value}</option>{/if}
    </select>
    <input
      type="color"
      title="A color of its own, written out"
      value={value && isHex(value) ? longHex(value) : "#808080"}
      disabled={readonly}
      onchange={(e) => onpick(e.currentTarget.value)}
    />
  </span>
{/snippet}

{#snippet lookFields(look: Look | null, kept: Look | null, style: boolean, onlook: (l: Look) => void)}
  <td>{@render colorPick(look?.background ?? null, kept?.background ?? null, true, (v) => onlook(changed(look, "background", v)))}</td>
  <td>{@render colorPick(look?.text ?? null, kept?.text ?? null, false, (v) => onlook(changed(look, "text", v)))}</td>
  <td>{@render colorPick(look?.border ?? null, kept?.border ?? null, true, (v) => onlook(changed(look, "border", v)))}</td>
  <td><input class="num" type="number" min="0" disabled={readonly} value={look?.radius ?? ""} placeholder={kept?.radius?.toString() ?? ""} title="Corner radius in pixels" onchange={(e) => onlook(changed(look, "radius", e.currentTarget.value))} /></td>
  <td><select disabled={readonly} value={look?.weight ?? ""} title="Weight of the text" onchange={(e) => onlook(changed(look, "weight", e.currentTarget.value))}>
    <option value="">{kept?.weight ? `as Clarity's: ${kept.weight}` : "not set"}</option>
    <option value="normal">normal</option>
    <option value="bold">bold</option>
  </select></td>
  <td><input class="num" type="number" min="6" max="200" disabled={readonly} value={look?.size ?? ""} placeholder={kept?.size?.toString() ?? ""} title="Text size in pixels" onchange={(e) => onlook(changed(look, "size", e.currentTarget.value))} /></td>
  {#if style}
    <td><select disabled={readonly} value={look?.font ?? ""} title="A font of the theme" onchange={(e) => onlook(changed(look, "font", e.currentTarget.value))}>
      <option value="">not set</option>
      {#each catalog.fonts as f (f)}<option value={f}>{f}</option>{/each}
    </select></td>
    <td><select disabled={readonly} value={look?.align ?? ""} title="Where the text sits" onchange={(e) => onlook(changed(look, "align", e.currentTarget.value))}>
      <option value="">not set</option>
      <option value="left">left</option>
      <option value="center">center</option>
      <option value="right">right</option>
    </select></td>
  {/if}
{/snippet}

<div class="form">
  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <h3>General</h3>
    <div class="grid two">
      <label for="theme-title">Title</label>
      <input id="theme-title" disabled={readonly} value={theme.title ?? ""} placeholder={theme.label} onchange={(e) => set(titleLine(e.currentTarget.value))} />
      <label for="theme-mode">Mode</label>
      <select id="theme-mode" disabled={readonly} value={theme.mode ?? ""} onchange={(e) => set(settingLine("mode", e.currentTarget.value))}>
        <option value="">as Clarity's: {base.mode ?? "system"}</option>
        <option value="system">light and dark, as the system is</option>
        <option value="light">light</option>
        <option value="dark">dark</option>
      </select>
      <label for="theme-padding">Padding</label>
      <input id="theme-padding" class="num" type="number" min="0" disabled={readonly} value={theme.padding ?? ""} placeholder={base.padding?.toString() ?? ""} onchange={(e) => set(settingLine("padding", number(e.currentTarget.value)))} />
      <label for="theme-border">Border width</label>
      <input id="theme-border" class="num" type="number" min="0" disabled={readonly} value={theme.border ?? ""} placeholder={base.border?.toString() ?? ""} onchange={(e) => set(settingLine("border", number(e.currentTarget.value)))} />
      <label for="theme-shadow">Shadow</label>
      <select id="theme-shadow" disabled={readonly} value={theme.shadow ?? ""} onchange={(e) => set(settingLine("shadow", e.currentTarget.value))}>
        <option value="">as Clarity's: {base.shadow ?? "none"}</option>
        <option value="none">none</option>
        <option value="flat">flat</option>
        <option value="raised">raised</option>
      </select>
    </div>
  </section>

  <section>
    <h3>Colors</h3>
    <table>
      <thead><tr><th>Color</th><th>Light</th><th>Dark</th><th></th></tr></thead>
      <tbody>
        {#each [...catalog.roles, ...extraColors] as name (name)}
          {@const mine = own(name)}
          {@const shown = mine ?? inherited(name)}
          <tr class:kept={!mine}>
            <td><code>{name}</code></td>
            <td>
              <input type="color" disabled={readonly} value={longHex(shown?.light ?? "#808080")} onchange={(e) => setColor(name, "light", e.currentTarget.value)} />
              <span class="hex">{shown?.light ?? ""}</span>
            </td>
            <td>
              <input type="color" disabled={readonly} value={longHex(shown?.dark ?? shown?.light ?? "#808080")} onchange={(e) => setColor(name, "dark", e.currentTarget.value)} />
              <span class="hex">{shown?.dark ?? "same"}</span>
            </td>
            <td>
              {#if mine && !readonly}
                <button class="link" onclick={() => set({ key: `color ${name}`, line: null })} title={catalog.roles.includes(name) ? "Take the theme's color away: Clarity's shows again" : "Take the color away"}>
                  {catalog.roles.includes(name) ? "Reset" : "Remove"}
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if !readonly}
      <form
        class="add"
        onsubmit={(e) => {
          e.preventDefault();
          if (newColor.trim()) set(colorLine(newColor.trim(), "#808080", null)).then(() => (newColor = ""));
        }}
      >
        <input bind:value={newColor} placeholder="brand" spellcheck="false" title="A color of the theme's own, for its styles and parts" />
        <button type="submit" disabled={!newColor.trim()}>Add color</button>
      </form>
    {/if}
  </section>

  <section>
    <h3>Fonts, sizes and corners</h3>
    <datalist id="theme-fonts">
      {#each FONT_CHOICES as f (f)}<option value={f}></option>{/each}
    </datalist>
    <div class="grid two">
      {#each catalog.fonts as role (role)}
        <label for="font-{role}">Font {role}</label>
        <input id="font-{role}" list="theme-fonts" disabled={readonly} value={roleValue(theme.fonts, role) ?? ""} placeholder={roleValue(base.fonts, role) ?? ""} onchange={(e) => set(fontLine(role, e.currentTarget.value))} />
      {/each}
      {#each catalog.sizes as role (role)}
        <label for="size-{role}">Size {role}</label>
        <input id="size-{role}" class="num" type="number" min="6" max="200" disabled={readonly} value={roleValue(theme.sizes, role) ?? ""} placeholder={roleValue(base.sizes, role) ?? ""} onchange={(e) => set(sizeLine(role, number(e.currentTarget.value)))} />
      {/each}
      {#each ["all", ...catalog.radii] as role (role)}
        <label for="radius-{role}">Radius {role}</label>
        <input id="radius-{role}" class="num" type="number" min="0" disabled={readonly} value={roleValue(theme.radii, role) ?? ""} placeholder={roleValue(base.radii, role) ?? ""} onchange={(e) => set(radiusLine(role, number(e.currentTarget.value)))} />
      {/each}
    </div>
  </section>

  <section>
    <h3>Parts</h3>
    <table class="looks">
      <thead><tr><th>Part</th><th>State</th><th>Background</th><th>Text</th><th>Border</th><th>Radius</th><th>Weight</th><th>Size</th><th></th></tr></thead>
      <tbody>
        {#each catalog.partKinds as kind (kind)}
          {#each statesOf(kind, catalog.states, catalog.selectable) as state (state ?? "")}
            {@const look = partLook(theme, kind, state)}
            <tr class:kept={!look}>
              <td>{state === null ? kind : ""}</td>
              <td>{state ?? "normal"}</td>
              {@render lookFields(look, partLook(base, kind, state), false, (l) => set(partLine(kind, state, l)))}
              <td>
                {#if look && !readonly}
                  <button class="link" onclick={() => set(partLine(kind, state, EMPTY_LOOK))} title="Take the theme's look away: Clarity's shows again">Reset</button>
                {/if}
              </td>
            </tr>
          {/each}
        {/each}
      </tbody>
    </table>
  </section>

  <section>
    <h3>Styles</h3>
    <table class="looks">
      <thead><tr><th>Style</th><th>Background</th><th>Text</th><th>Border</th><th>Radius</th><th>Weight</th><th>Size</th><th>Font</th><th>Align</th><th></th></tr></thead>
      <tbody>
        {#each theme.styles as style (style.name.name)}
          <tr>
            <td><code>{style.name.name}</code></td>
            {@render lookFields(style.look, null, true, (l) => set(styleLine(style.name.name, l)))}
            <td>
              {#if !readonly}
                <button class="link" onclick={() => set({ key: `style ${style.name.name}`, line: null })}>Remove</button>
              {/if}
            </td>
          </tr>
        {:else}
          <tr><td colspan="10" class="muted">No styles: a style is a look a part takes by name, such as a primary button.</td></tr>
        {/each}
      </tbody>
    </table>
    {#if !readonly}
      <form
        class="add"
        onsubmit={(e) => {
          e.preventDefault();
          if (newStyle.trim()) set(styleLine(newStyle.trim(), { ...EMPTY_LOOK, background: "accent", text: "on_accent" })).then(() => (newStyle = ""));
        }}
      >
        <input bind:value={newStyle} placeholder="primary" spellcheck="false" />
        <button type="submit" disabled={!newStyle.trim()}>Add style</button>
      </form>
    {/if}
  </section>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
    font-size: 12px;
  }
  h3 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  .grid.two {
    display: grid;
    grid-template-columns: max-content minmax(0, 360px);
    gap: 4px 10px;
    align-items: center;
  }
  table {
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-weight: 600;
    color: var(--text-muted);
    padding: 2px 6px;
  }
  td {
    padding: 2px 6px;
    vertical-align: middle;
  }
  .looks :global(select) {
    max-width: 150px;
  }
  tr.kept .hex,
  tr.kept td:first-child {
    color: var(--text-muted);
  }
  .hex {
    font-family: var(--mono);
    margin-left: 4px;
  }
  .pick {
    display: inline-flex;
    gap: 2px;
    align-items: center;
    margin-right: 4px;
  }
  .pick input[type="color"] {
    width: 22px;
    height: 20px;
    padding: 0;
    border: none;
    background: none;
  }
  .num {
    width: 64px;
  }
  .add {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
  }
  .error {
    color: var(--error);
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
  }
  code {
    font-family: var(--mono);
  }
</style>
