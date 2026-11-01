<script lang="ts">
  import ChartView from "../parts/ChartView.svelte";
  import { groupsOf, type ByKind } from "../parts/charts.ts";
  import type { JsonValue } from "../generated/serde_json/JsonValue.ts";
  import { displayValue, findEnum, findField } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";
  import { declareView, viewFilled } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // What this chart shows of its list, for the runtime to fill in (DESIGN 1.7):
  // declared when it shows in a running application.
  $effect(() => {
    const collection = ctx.collection;
    if (!ctx.live || ctx.design || !p.name || !p.bind || !collection || !p.by) return;
    const own = collection.entity;
    const type = findField(own, p.by)?.type;
    const byKind = type?.kind === "enum" ? "enum" : type?.kind === "bool" ? "bool" : type && ["integer", "decimal", "money", "duration", "id"].includes(type.kind) ? "number" : type?.kind === "date" || type?.kind === "dateTime" ? "date" : "text";
    const values = type?.kind === "enum" ? (findEnum(ctx.preview, type.name.name)?.variants ?? []).map((v) => v.name.name) : [];
    const measure = p.measure;
    declareView(ctx, { kind: "chart", list: p.bind, by: p.by, byKind, measure: measure?.kind ?? "count", of: measure && measure.kind !== "count" ? measure.field : null, values });
  });

  // A chart (U1c): the list's rows grouped by a field, each group's number drawn.
  const own = $derived(ctx.collection?.entity);
  const byField = $derived(own && p.by ? findField(own, p.by) : undefined);
  const measure = $derived(p.measure);
  const ofField = $derived(own && measure && measure.kind !== "count" ? findField(own, measure.field) : undefined);
  const byType = $derived(byField?.type);
  const by: ByKind = $derived(
    byType?.kind === "enum"
      ? "enum"
      : byType?.kind === "bool"
        ? "bool"
        : byType && ["integer", "decimal", "money", "duration", "id"].includes(byType.kind)
          ? "number"
          : byType?.kind === "date" || byType?.kind === "dateTime"
            ? "date"
            : "text",
  );
  const order = $derived(byType?.kind === "enum" ? (findEnum(ctx.preview, byType.name.name)?.variants ?? []).map((v) => v.name.name) : []);
  const rows = $derived(
    (ctx.collection?.rows ?? []).map((r) => {
      const value = byField ? r[byField.name.name] : null;
      // A reference groups by the text it shows.
      return { by: byType?.kind === "ref" && value !== null && value !== undefined ? displayValue(ctx.preview, byField!, value) : value, of: ofField ? r[ofField.name.name] : null };
    }),
  );
  // The runtime's groups when it filled them in (DESIGN 1.7); a reference then groups by its record, labelled with its text.
  const runtimeGroups = $derived.by(() => {
    const filled = viewFilled(ctx);
    return Array.isArray(filled?.groups) ? (filled.groups as { key: unknown; value: number }[]) : null;
  });
  const groups = $derived(runtimeGroups ?? groupsOf(rows, by, measure?.kind ?? "count", order));
</script>

<ChartView
  kind={p.chart!}
  label={p.name ?? undefined}
  groups={groups.map((g) => ({
    label: byField && (byType?.kind !== "ref" || runtimeGroups) ? displayValue(ctx.preview, byField, g.key as JsonValue) : String(g.key),
    value: g.value,
    text: ofField && measure && measure.kind !== "count" ? displayValue(ctx.preview, ofField, g.value) : String(g.value),
  }))}
/>
