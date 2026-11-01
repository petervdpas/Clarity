<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { keyFile, tr } from "../screen/model.ts";
  import { pictureUrl } from "../screen/pictures.svelte.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const preview = $derived(ctx.preview);
  const named = $derived(p.source ?? (p.caption ? (keyFile(preview, p.caption) ?? tr(preview, p.caption)) : null));
  const picture = $derived(named ? pictureUrl(named) : null);
</script>

{#if picture}
  <img class="picture" src={picture} alt={p.tooltip ? tr(preview, p.tooltip) : ""} />
{:else}
  <div class="placeholder image">
    <span>Image</span>
    <code>{named ?? "no source"}</code>
  </div>
{/if}

<style>
  .picture {
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: contain;
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: var(--border-width) dashed var(--border-strong);
    border-radius: var(--radius-panel);
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .placeholder code {
    font-family: var(--mono);
    font-size: var(--font-size-small);
  }
</style>
