<script lang="ts">
  // Clarity's own settings, kept for the person rather than in a project:
  // for now its theme. A choice shows at once.

  import { IDE_THEMES, type IdeTheme } from "./ideTheme";
  import { ideTheme, setIdeTheme } from "./ideTheme.svelte";

  let { onclose }: { onclose: () => void } = $props();

  /** A strip of each theme's colors: its binding, its pages and its accent. */
  const SWATCHES: Record<IdeTheme, string[]> = {
    system: ["#20242c", "#ffffff", "#1d2129", "#1f5fbf"],
    light: ["#20242c", "#eef0f3", "#ffffff", "#1f5fbf"],
    dark: ["#0d0f13", "#15181e", "#1d2129", "#6fa3f2"],
    "cream-green": ["#23402b", "#efe9d4", "#fbf8ee", "#2e6b3a"],
  };
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-labelledby="app-settings-title">
    <h2 id="app-settings-title">Clarity settings</h2>
    <p class="muted">For you on this computer, in every project.</p>

    <fieldset>
      <legend>Theme</legend>
      {#each IDE_THEMES as t (t.value)}
        <label class="theme" class:chosen={ideTheme.value === t.value}>
          <input type="radio" name="ide-theme" checked={ideTheme.value === t.value} onchange={() => setIdeTheme(t.value)} />
          <span class="swatch" aria-hidden="true">
            {#each SWATCHES[t.value] as color (color)}<span style:background={color}></span>{/each}
          </span>
          <span class="words">
            <strong>{t.label}</strong>
            <span class="muted">{t.note}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="buttons">
      <button type="button" class="primary" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    z-index: 50;
  }

  .dialog {
    width: min(440px, calc(100vw - 32px));
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.35);
  }

  h2 {
    margin: 0;
    font-size: 15px;
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  legend {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .theme {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .theme.chosen {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .swatch {
    display: flex;
    flex-shrink: 0;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    overflow: hidden;
  }

  .swatch span {
    width: 14px;
    height: 26px;
  }

  .words {
    display: flex;
    flex-direction: column;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
  }
</style>
