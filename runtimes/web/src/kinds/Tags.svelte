<script lang="ts">
  import type { PartContext } from "./context.ts";
  import TagsInput from "../parts/TagsInput.svelte";
  import type { Item, ItemKind } from "../screen/listFields.ts";
  import { findEnum, itemText, ownWord, tr } from "../screen/model.ts";
  import { wiredCall } from "../screen/calls.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const live = $derived(ctx.live);
  const bound = $derived(ctx.bound);
  const preview = $derived(ctx.preview);

  const type = $derived(bound?.field.type);
  const listed = $derived(type?.kind === "values" ? type.item : null);
</script>

<TagsInput
  id={ctx.id}
  items={(bound ? ctx.value : []) as Item[] | null}
  kind={(listed?.kind ?? "text") as ItemKind}
  choices={listed?.kind === "enum" ? (findEnum(preview, listed.name.name)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(preview, v.label) })) : []}
  show={(item) => (bound ? itemText(preview, bound.field, item) : String(item))}
  words={{ add: ownWord(preview, "Add"), remove: ownWord(preview, "Remove") }}
  describedBy={ctx.describedBy}
  disabled={ctx.design || ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
  onadd={live?.item && p.name && p.bind
    ? (typed) =>
        live.item!(
          p.name!,
          {
            path: p.bind!,
            item: listed?.kind === "enum" ? "enum" : (listed?.kind ?? "text"),
            values: listed?.kind === "enum" ? (findEnum(preview, listed.name.name)?.variants ?? []).map((v) => v.name.name) : [],
            typed,
          },
          wiredCall(p.handlers, "change"),
        )
    : undefined}
/>
