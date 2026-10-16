<script lang="ts">
  // Backend, UI and targets of a project, with the choices the Rust
  // catalog offers. Used by the New project dialog and the settings.

  import type { Backend } from "./generated/Backend";
  import type { Catalog } from "./generated/Catalog";
  import type { Stack } from "./generated/Stack";
  import type { Target } from "./generated/Target";
  import type { Ui } from "./generated/Ui";

  let {
    catalog,
    stack,
    onchange,
  }: {
    catalog: Catalog;
    stack: Stack;
    onchange: (stack: Stack) => void;
  } = $props();

  const uis = $derived(catalog.backends.find((b) => b.id === stack.backend)?.uis ?? []);

  function setBackend(backend: Backend) {
    const offered = catalog.backends.find((b) => b.id === backend)?.uis ?? [];
    // Keep the UI when the new backend offers it, else take its usual one.
    const ui = offered.includes(stack.ui) ? stack.ui : (offered[0] ?? stack.ui);
    onchange({ ...stack, backend, ui });
  }

  function toggleTarget(target: Target, on: boolean) {
    const targets = catalog.targets.map((t) => t.id).filter((t) => (t === target ? on : stack.targets.includes(t)));
    onchange({ ...stack, targets });
  }

  function uiLabel(ui: Ui): string {
    return catalog.uis.find((u) => u.id === ui)?.label ?? ui;
  }
</script>

<div class="stack">
  <label>
    <span>Backend</span>
    <select value={stack.backend} onchange={(e) => setBackend(e.currentTarget.value as Backend)}>
      {#each catalog.backends as b (b.id)}<option value={b.id}>{b.label}</option>{/each}
    </select>
  </label>
  <label>
    <span>UI</span>
    <select value={stack.ui} onchange={(e) => onchange({ ...stack, ui: e.currentTarget.value as Ui })}>
      {#each uis as u (u)}<option value={u}>{uiLabel(u)}</option>{/each}
    </select>
  </label>
  <fieldset>
    <legend>Targets</legend>
    {#each catalog.targets as t (t.id)}
      <label class="check">
        <input type="checkbox" checked={stack.targets.includes(t.id)} onchange={(e) => toggleTarget(t.id, e.currentTarget.checked)} />
        {t.label}
      </label>
    {/each}
  </fieldset>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label,
  fieldset {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  fieldset {
    flex-direction: row;
    gap: 14px;
    border: none;
    padding: 0;
    margin: 0;
  }

  legend {
    padding: 0;
    margin-bottom: 3px;
  }

  label.check {
    flex-direction: row;
    align-items: center;
    gap: 5px;
    color: var(--text);
  }
</style>
