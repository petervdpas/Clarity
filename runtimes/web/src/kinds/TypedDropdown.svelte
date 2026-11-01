<script lang="ts">
  import type { PartContext } from "./context.ts";
  import TypeAheadInput from "../parts/TypeAheadInput.svelte";
  import { suggestions } from "../screen/dropdownOptions.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const live = $derived(ctx.live);
  const preview = $derived(ctx.preview);
  const value = $derived(ctx.value);

  const owner = $derived(p.bind ? ctx.data[p.bind.split(".")[0]]?.entity.name.name : undefined);
  const fieldName = $derived(ctx.bound?.field.name.name);
  const known = $derived(
    owner && fieldName ? (live ? live.fieldValues(owner, fieldName) : (preview.samples[owner] ?? []).map((r) => r[fieldName]).filter((v): v is string => typeof v === "string")) : [],
  );
</script>

<!-- Any text, the values the field already has offered (U1c). -->
<TypeAheadInput
  id={ctx.id}
  text={typeof value === "string" ? value : ""}
  free
  offer={(typed) =>
    live?.findOptions && owner && fieldName
      ? live.findOptions({ kind: "typed", entity: owner, field: fieldName, typed }).then((list) => list.map((v) => ({ key: v, cells: [v] })))
      : suggestions(known, typed).map((v) => ({ key: v, cells: [v] }))}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onpick={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
  oncommit={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
/>
