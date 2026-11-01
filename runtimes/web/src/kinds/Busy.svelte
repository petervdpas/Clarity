<script lang="ts">
  import type { PartContext } from "./context.ts";
  let { ctx }: { ctx: PartContext } = $props();
</script>

<span class="busy" role="status" id={ctx.id}><span class="spinner"></span>{ctx.caption ?? ""}</span>

<style>
  .busy {
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
