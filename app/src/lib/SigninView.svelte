<script lang="ts">
  // The project's sign-in setup (docs/APP-CAPABILITIES-PLAN.md step 4),
  // opened from the project tree: the project file's `signin` block. Who signs
  // in, on which targets, by which strategies, what the sign-in part shows and
  // whether it asks to keep people signed in. A window or the splash only
  // places the sign-in part; what it offers is set here.

  import SigninSettings from "./SigninSettings.svelte";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { ProjectFile } from "@clarity/ui/generated/ProjectFile.ts";
  import type { Settings } from "@clarity/ui/generated/Settings.ts";

  let {
    project,
    preview,
    onsave,
  }: {
    project: ProjectFile;
    preview: Preview;
    onsave: (settings: Settings) => void;
  } = $props();

  /** The stored entities, one of which holds the people. */
  const entities = $derived(preview.project.files.flatMap((f) => f.entities).filter((e) => !e.settings && !e.record));
</script>

<div class="view">
  <header>
    <h1>Sign-in</h1>
    <span class="muted">the signin block of the project file</span>
  </header>
  <p class="intro">
    Who uses the program and how they sign in. A sign-in window, or the splash, holds the sign-in part; it offers what is set here.
  </p>
  {#if project.settings}
    <SigninSettings
      signin={project.settings.signin ?? null}
      strategies={project.catalog.strategies}
      {entities}
      targets={project.settings.stack.targets}
      onchange={(signin) => project.settings && onsave({ ...project.settings, signin })}
    />
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 24px;
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-size: 18px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .intro {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
