<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { progressFull } from "../screen/smallParts.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const full = $derived(progressFull(ctx.value, p.max));
</script>

<span class="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(full * 100)} id={ctx.id}>
  <span class="progress-fill" style:width="{full * 100}%"></span>
</span>

<style>
  .progress {
    flex: 1;
    align-self: center;
    height: 8px;
    border-radius: 4px;
    background: var(--panel-alt);
    border: var(--border-width, 1px) solid var(--border);
    overflow: hidden;
  }

  .progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }
</style>
