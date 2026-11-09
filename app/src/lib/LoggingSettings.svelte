<script lang="ts">
  // The Logging section of Clarity settings, for every project: whether the
  // IDE writes a log of what it does (the designer's steps, the errors of its
  // commands), a new file each start with the last five kept.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { LoggingView } from "@clarity/ui/generated/LoggingView.ts";

  let view = $state<LoggingView | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      view = await invoke<LoggingView>("logging_view");
    } catch (err) {
      error = String(err);
    }
  }

  async function set(enabled: boolean) {
    error = null;
    try {
      view = await invoke<LoggingView>("set_logging", { enabled });
    } catch (err) {
      error = String(err);
    }
  }

  async function openFolder() {
    error = null;
    try {
      await invoke("open_log_folder");
    } catch (err) {
      error = String(err);
    }
  }

  $effect(() => {
    void load();
  });
</script>

<div class="logging">
  <p class="muted">A log of what Clarity does, to follow a problem afterwards. It holds no project contents beyond the names of parts.</p>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  {#if view}
    <fieldset>
      <legend>Log</legend>
      <label class="check">
        <input type="checkbox" checked={view.enabled} onchange={(e) => set(e.currentTarget.checked)} />
        <span>Write a log</span>
      </label>
      <p class="muted">Holds from now on and in the next starts. Each start begins a new file; the last five are kept.</p>
      <div class="other">
        <input type="text" readonly value={view.enabled ? view.file : view.folder} aria-label="Log file" title={view.enabled ? view.file : view.folder} />
        <button onclick={openFolder}>Open log folder</button>
      </div>
    </fieldset>
  {/if}
</div>

<style>
  .logging {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .other {
    display: flex;
    gap: 6px;
  }
  .other input {
    flex: 1;
    font-family: var(--mono);
    cursor: default;
  }
</style>
