<script lang="ts">
  // Backend, UI and targets of a project, with the choices the Rust
  // catalog offers, as rows of a .form-grid. Used by the New project dialog
  // and the settings.

  import type { Backend } from "./generated/Backend";
  import type { Catalog } from "./generated/Catalog";
  import type { Stack } from "./generated/Stack";
  import type { Target } from "./generated/Target";
  import type { Ui } from "./generated/Ui";
  import { toolsNote } from "./stackTools";
  import { checkTools, stackTools } from "./tools.svelte";

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

  // Whether this machine can build the stack (P3).
  $effect(() => checkTools());
  const note = $derived(toolsNote(stackTools.all, stack.backend, stack.ui));

  function uiLabel(ui: Ui): string {
    return catalog.uis.find((u) => u.id === ui)?.label ?? ui;
  }
</script>

<!-- The rows go into the surrounding .form-grid. -->
<span class="form-label">Backend</span>
<select value={stack.backend} onchange={(e) => setBackend(e.currentTarget.value as Backend)} aria-label="Backend">
  {#each catalog.backends as b (b.id)}<option value={b.id}>{b.label}</option>{/each}
</select>
<span class="form-label">UI</span>
<select value={stack.ui} onchange={(e) => onchange({ ...stack, ui: e.currentTarget.value as Ui })} aria-label="UI">
  {#each uis as u (u)}<option value={u}>{uiLabel(u)}</option>{/each}
</select>
<span class="form-label">Targets</span>
<div class="form-checks" role="group" aria-label="Targets">
  {#each catalog.targets as t (t.id)}
    <label>
      <input type="checkbox" checked={stack.targets.includes(t.id)} onchange={(e) => toggleTarget(t.id, e.currentTarget.checked)} />
      {t.label}
    </label>
  {/each}
</div>
<span class="form-label">Build tools</span>
<div class="tools">
  {#if note}
    <span class="note {note.level}">{note.text}</span>
  {:else}
    <span class="note">{stackTools.checking ? "Looking for the build tools..." : "Not checked"}</span>
  {/if}
  <button type="button" disabled={stackTools.checking} onclick={() => checkTools(true)}>Check again</button>
</div>

<style>
  .tools {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .note {
    flex: 1;
    font-size: 12px;
    color: var(--text-muted);
    user-select: text;
  }

  .note.warning {
    color: var(--warning, var(--text));
  }

  button {
    flex: none;
  }
</style>
