<script lang="ts">
  import Table from "../parts/Table.svelte";
  import { ownWord } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";
  import { declareView, reordered, reorders, tableColumnsOf, viewAction, viewFilled } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const tableColumns = $derived(tableColumnsOf(ctx));
  const own = $derived(ctx.collection?.entity);
  const parent = $derived(own?.fields.find((f) => f.type.kind === "ref" && f.type.entity.name === own.name.name)?.name.name ?? "parent");

  // What this tree shows of its list, for the runtime to fill in (DESIGN 1.7):
  // declared when it shows in a running application.
  $effect(() => {
    const collection = ctx.collection;
    if (!ctx.live || ctx.design || !p.name || !p.bind || !collection) return;
    const own = collection.entity;
    const parent = own.fields.find((f) => f.type.kind === "ref" && f.type.entity.name === own.name.name)?.name.name ?? "parent";
    declareView(ctx, { kind: "tree", list: p.bind, parent, key: "id" });
  });
</script>

<!-- A tree (U1c): the rows under their parents, by the one reference to their own kind. -->
<Table
  columns={tableColumns}
  rows={ctx.collection?.rows ?? []}
  selected={ctx.selectedRow ?? null}
  design={ctx.design}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  resizable={p.resizable}
  header={p.kind === "treeTable"}
  tree={{
    parent,
    open: ownWord(ctx.preview, "Expand"),
    close: ownWord(ctx.preview, "Collapse"),
    // In the designer a sample reference holds the text of the row it points at.
    key: ctx.live ? "id" : own?.fields.find((f) => f.type.kind === "text")?.name.name,
  }}
  remember={p.name ? `clarity-columns.${ctx.page.name.name}.${p.name}` : null}
  onselect={(row) => ctx.selectRow(row)}
  onopen={() => ctx.fire("open")}
  onedit={() => ctx.fire("change")}
  onchanged={(record, field, before, after) => {
    // The cell's path in the window's data, by the record's place in its list, for undo.
    const at = (ctx.collection?.rows ?? []).indexOf(record);
    if (ctx.live && !ctx.design && p.name && p.bind && at >= 0) ctx.live.edited?.(p.name);
  }}
  reorder={reorders(ctx)}
  onreorder={(move) => reordered(ctx, move)}
  view={viewFilled(ctx) as never}
  onview={viewAction(ctx)}
/>
