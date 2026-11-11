<script lang="ts">
  // The options of a file format from the catalog, a box for a flag and a
  // field for a text: under a file source in Settings, and on a mapping's page.

  import type { FormatChoice } from "@clarity/ui/generated/FormatChoice.ts";

  let {
    format,
    value,
    onset,
    skip = [],
  }: {
    format: FormatChoice | undefined;
    /** The option's value as text: `true` for a flag that is on, empty when not given. */
    value: (name: string) => string;
    /** Sets an option; an empty value leaves it out. */
    onset: (name: string, value: string) => void;
    /** Options shown elsewhere on the page (a mapping's `at`). */
    skip?: string[];
  } = $props();
</script>

<div class="format-options">
  {#each (format?.options ?? []).filter((o) => !skip.includes(o.name)) as option (option.name)}
    {#if option.flag}
      <label title={option.help}>
        <input type="checkbox" checked={value(option.name) === "true"} onchange={(e) => onset(option.name, e.currentTarget.checked ? "true" : "")} />
        {option.label}
      </label>
    {:else}
      <label title={option.help}>
        {option.label}
        <input class="mono" value={value(option.name)} onchange={(e) => onset(option.name, e.currentTarget.value)} />
      </label>
    {/if}
  {/each}
</div>

<style>
  .format-options {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 4px;
  }

  .format-options label {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
  }

  .format-options input.mono {
    width: 7em;
  }
</style>
