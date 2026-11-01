<script lang="ts">
  import type { PartContext } from "./context.ts";
  import FileInput, { type RecordFileValue } from "../parts/FileInput.svelte";
  import { ownWord } from "../screen/model.ts";
  import { boundEntity } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();

  const kept = $derived((ctx.bound ? ctx.value : null) as RecordFileValue | null);

  /** The bytes of a record's file, as base64: new ones as chosen, a stored one fetched; then `use` them. */
  async function withFileData(file: RecordFileValue | null, use: (name: string, data: string) => Promise<void> | undefined) {
    const { live, design, bound } = ctx;
    const entity = boundEntity(ctx);
    if (!file || !live || design) return;
    const data = file.data ?? (file.stored && bound && entity ? await live.fileData(entity, Number(bound.record.id ?? 0), bound.field.name.name) : null);
    if (data) await use(file.name, data);
  }
</script>

<FileInput
  id={ctx.id}
  value={kept}
  words={{
    none: ownWord(ctx.preview, "No file"),
    choose: ownWord(ctx.preview, "Choose a file"),
    open: ownWord(ctx.preview, "Open"),
    save: ownWord(ctx.preview, "Save a copy"),
    remove: ownWord(ctx.preview, "Remove the file"),
  }}
  describedBy={ctx.describedBy}
  disabled={ctx.design || ctx.disabled}
  readonly={ctx.readonly}
  onchoose={(name, data) => {
    ctx.setValue({ name, data });
    ctx.fire("change");
  }}
  onremove={() => {
    ctx.setValue(null);
    ctx.fire("change");
  }}
  onopen={() => withFileData(kept, (name, data) => ctx.live?.openFile(name, data))}
  onsave={() => withFileData(kept, (name, data) => ctx.live?.saveFile(name, data))}
/>
