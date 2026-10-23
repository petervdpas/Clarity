<script lang="ts" module>
  /** A record's file as the backend sends it: stored, new (base64), or a file of its store; always with its name. */
  export type RecordFileValue = { name: string; stored?: boolean; data?: string; file?: string };
</script>

<script lang="ts">
  // Clarity's file input on the web, the same as on Avalonia (U1c): the
  // name of a record's file, or that it has none, with buttons to choose
  // one, open it in the program the system uses for its kind, save a copy
  // in the person's Downloads folder, and remove it. Choosing and removing
  // are changes; opening and saving a copy are not.

  let {
    value,
    words,
    id = undefined,
    describedBy = undefined,
    disabled = false,
    readonly = false,
    onchoose,
    onopen,
    onsave,
    onremove,
  }: {
    value: RecordFileValue | null | undefined;
    /** Clarity's own words, in the language shown. */
    words: { none: string; choose: string; open: string; save: string; remove: string };
    id?: string;
    describedBy?: string;
    disabled?: boolean;
    readonly?: boolean;
    /** A file was chosen: its name and its bytes as base64. */
    onchoose: (name: string, data: string) => void;
    onopen: () => void;
    onsave: () => void;
    onremove: () => void;
  } = $props();

  let picker = $state<HTMLInputElement | null>(null);
  // A file just linked from a store opens once the record is saved.
  const reachable = $derived(!!value && (value.stored === true || typeof value.data === "string"));

  async function chosen(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    e.currentTarget.value = "";
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    let text = "";
    for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    onchoose(file.name, btoa(text));
  }
</script>

<span class="file-input" {id} aria-describedby={describedBy}>
  <span class="name" class:none={!value}>{value?.name ?? words.none}</span>
  {#if value}
    <button type="button" disabled={disabled || !reachable} onclick={onopen}>{words.open}</button>
    <button type="button" disabled={disabled || !reachable} onclick={onsave}>{words.save}</button>
  {/if}
  {#if !readonly}
    <button type="button" {disabled} onclick={() => picker?.click()}>{words.choose}</button>
    <button type="button" disabled={disabled || !value} onclick={onremove}>{words.remove}</button>
    <input bind:this={picker} type="file" hidden onchange={chosen} />
  {/if}
</span>

<style>
  .file-input {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    height: 100%;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    color: var(--text-muted);
  }
  button {
    flex: none;
  }
</style>
