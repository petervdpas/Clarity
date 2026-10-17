<script lang="ts">
  // A small window drawn with a theme, so its colors, font and styles can be
  // judged while the theme is written. It holds the same classes the
  // application does; the CSS is the generator's (ThemeStyles).

  import type { ThemeDecl } from "./generated/ThemeDecl";
  import { themeArea } from "./model";

  let { theme }: { theme: ThemeDecl } = $props();

  const area = $derived(themeArea(theme));
</script>

<div class="sample {area.class}" data-scheme={area.scheme}>
  <div class="panel">
    <strong>Customer</strong>
    <label>Name <input value="Ada Lovelace" readonly /></label>
    <span class="muted">Help text and other quiet text</span>
    <span class="error">A message at an input</span>
    <div class="buttons">
      <button>Plain</button>
      {#each theme.styles as style (style.class)}
        <button class={style.class}>{style.name.name}</button>
      {/each}
    </div>
  </div>
</div>

<style>
  .sample {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
  }

  .panel {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .muted {
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
</style>
