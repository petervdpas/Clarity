<script lang="ts">
  // A DevTools build as it goes: how it stands, who pressed it, and its
  // last lines, the newest kept in view while it builds.
  import type { JobView } from "@clarity/ui/generated/JobView.ts";
  import { stateText } from "./devtools";

  let { job }: { job: JobView } = $props();
  let box: HTMLPreElement | undefined = $state();

  $effect(() => {
    void job.lines.length;
    if (box) box.scrollTop = box.scrollHeight;
  });
</script>

<div class="build {job.state}">
  <p>
    <strong>{stateText(job)}</strong>
    {#if job.by}<span class="muted">pressed for {job.by}</span>{/if}
    {#if job.why}<span class="why">{job.why}</span>{/if}
  </p>
  {#if job.lines.length}<pre bind:this={box}>{job.lines.join("\n")}</pre>{/if}
</div>

<style>
  .build {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
  }
  .build.running,
  .build.waiting {
    border-color: var(--accent);
  }
  .build.failed {
    border-color: var(--error);
  }
  p {
    margin: 0 0 4px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12px;
  }
  .muted {
    color: var(--text-muted);
  }
  .why {
    color: var(--error);
  }
  pre {
    margin: 0;
    max-height: 260px;
    overflow: auto;
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
