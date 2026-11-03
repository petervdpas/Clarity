<script lang="ts">
  // A custom part (DESIGN 5.19): a library in the stack's own language draws
  // it on Clarity.UI's canvas. On this screen (the design canvas, the
  // Preview and a web target) it is a box with its name, or its picture.

  import { pictureUrl } from "../screen/pictures.svelte.ts";

  let {
    kind,
    preview: picture,
  }: {
    kind: string;
    /** Its picture of the resources, from its declaration's `preview`. */
    preview: string | null;
  } = $props();

  const pictured = $derived(picture ? pictureUrl(picture) : null);
</script>

<div class="custom-part">
  {#if pictured}
    <img src={pictured} alt={kind} />
  {:else}
    <span>{kind}</span>
  {/if}
</div>

<style>
  .custom-part {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--border, #999);
    box-sizing: border-box;
    overflow: hidden;
  }
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  span {
    opacity: 0.7;
  }
</style>
