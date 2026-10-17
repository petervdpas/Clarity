<script lang="ts">
  // The CSS of the project's themes in the designer's page: the files the
  // generator writes for the application, read again when the project
  // changes, so the canvas and theme samples look as the application will.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "./generated/Preview";

  let { preview }: { preview: Preview } = $props();

  let css = $state("");

  $effect(() => {
    void preview;
    invoke<string>("theme_styles")
      .then((text) => (css = text))
      .catch(() => (css = ""));
  });
</script>

<svelte:head>
  {@html `<style data-clarity-themes>${css}</style>`}
</svelte:head>
