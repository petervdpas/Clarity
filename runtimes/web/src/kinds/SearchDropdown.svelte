<script lang="ts">
  import type { PartContext } from "./context.ts";
  import TypeAheadInput from "../parts/TypeAheadInput.svelte";
  import { searched, type RefOption } from "../screen/dropdownOptions.ts";
  import { displayValue, findEntity, findField } from "../screen/model.ts";
  import type { JsonValue } from "../generated/serde_json/JsonValue.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const live = $derived(ctx.live);
  const preview = $derived(ctx.preview);
  const bound = $derived(ctx.bound);
  const value = $derived(ctx.value);
</script>

{#if bound?.field.type.kind === "ref"}
  <!-- A record among many, filtered as it is typed, with the fields it shows (U1c). -->
  {@const target = bound.field.type.entity.name}
  {@const records: RefOption[] = live ? live.refOptions(target) : (preview.displays[target] ?? []).map((text, i) => ({ id: i + 1, text }))}
  {@const targetEntity = findEntity(preview, target)}
  {@const shownFields = p.show.map((f) => (targetEntity ? findField(targetEntity, f) : undefined))}
  {@const cellsOf = (o: RefOption) => p.show.map((f, i) => (shownFields[i] ? displayValue(preview, shownFields[i]!, (o.fields?.[f] ?? null) as JsonValue) : ""))}
  <TypeAheadInput
    id={ctx.id}
    text={records.find((o) => o.id === value || o.text === value)?.text ?? ""}
    offer={(typed) =>
      live?.findOptions
        ? live.findOptions({ kind: "search", entity: target, show: p.show, typed }).then((list) => list.map((o) => ({ key: String(o.id), cells: [o.text, ...cellsOf(o)] })))
        : searched(records, typed, cellsOf).map((o) => ({ key: String(o.id), cells: [o.text, ...cellsOf(o)] }))}
    tabindex={p.tabOrder ?? undefined}
    describedBy={ctx.describedBy}
    invalid={ctx.error !== null}
    disabled={ctx.disabled}
    readonly={ctx.readonly}
    onpick={(key) => {
      const picked = records.find((o) => String(o.id) === key);
      ctx.setValue(live ? (picked?.id ?? null) : (picked?.text ?? null));
      ctx.fire("change");
    }}
    onclear={() => {
      ctx.setValue(null);
      ctx.fire("change");
    }}
  />
{/if}
