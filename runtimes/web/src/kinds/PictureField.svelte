<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { ownWord } from "../screen/model.ts";
  import { isStored, linkedFile, pictureFromFile, pictureUrl as recordPictureUrl } from "../screen/recordPictures.ts";
  import { boundEntity as boundEntityOf } from "./shared.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const preview = $derived(ctx.preview);
  const live = $derived(ctx.live);
  const bound = $derived(ctx.bound);
  const value = $derived(ctx.value);
  const caption = $derived(ctx.caption);

  // A picture field: its stored picture comes on its own; a new one is
  // chosen from a file and kept in the field until the record is saved.
  const boundEntity = $derived(boundEntityOf(ctx));
  const canLoadPicture = $derived(!!ctx.live && !ctx.design && !ctx.readonly && !ctx.disabled);
  let fileInput = $state<HTMLInputElement | null>(null);
  let pictureProblem = $state(false);
  const linked = $derived(linkedFile(value));
  const fetched = $derived(
    !live || !bound
      ? undefined
      : linked && bound.field.modifiers.store
        ? live.storeFile(bound.field.modifiers.store, linked)
        : boundEntity && isStored(value)
          ? live.picture(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name)
          : undefined,
  );
  const shownPicture = $derived(recordPictureUrl(value, fetched));

  function forgetFetched() {
    if (live && bound && boundEntity) live.forgetPicture(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name);
  }

  async function pictureChosen(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = "";
    if (!file) return;
    const picture = pictureFromFile(new Uint8Array(await file.arrayBuffer()));
    pictureProblem = picture === null;
    if (!picture) return;
    forgetFetched();
    ctx.setValue(picture);
  }

  function removePicture() {
    pictureProblem = false;
    forgetFetched();
    ctx.setValue(null);
  }
</script>

<div class="picture-field">
  {#if shownPicture}
    <img class="picture" src={shownPicture} alt={caption ?? ""} />
  {:else}
    <div class="placeholder image" class:problem={pictureProblem}>
      <span>{ownWord(preview, "No picture")}</span>
    </div>
  {/if}
  {#if canLoadPicture}
    <div class="picture-actions">
      <button type="button" onclick={() => fileInput?.click()}>{ownWord(preview, "Load a picture")}</button>
      {#if value}
        <button type="button" onclick={removePicture}>{ownWord(preview, "Remove the picture")}</button>
      {/if}
    </div>
    <input bind:this={fileInput} type="file" accept="image/png,image/jpeg,image/gif,image/bmp,image/webp" hidden onchange={pictureChosen} />
  {/if}
</div>

<style>
  .picture {
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: contain;
  }

  .picture-field {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
  }

  /* The buttons show when the pointer or the keyboard is on the picture. */
  .picture-actions {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    justify-content: center;
    gap: 4px;
    padding: 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .picture-field:hover .picture-actions,
  .picture-field:focus-within .picture-actions {
    opacity: 1;
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

  .placeholder.problem {
    outline: 2px solid var(--error);
  }
</style>
