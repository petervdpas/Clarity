<script lang="ts">
  import Table from "../parts/Table.svelte";
  import { ownWord } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";
  import { declareView, reordered, reorders, tableColumnsOf, viewAction, viewFilled } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const tableColumns = $derived(tableColumnsOf(ctx));

  // What this table shows of its list, for the runtime to fill in (DESIGN 1.7):
  // declared when it shows in a running application.
  $effect(() => {
    if (!ctx.live || ctx.design || !p.name || !p.bind || !ctx.collection) return;
    declareView(ctx, {
      kind: "table",
      list: p.bind,
      columns: tableColumns.map((c) => ({ field: c.field, kind: c.kind, choices: (c.choices ?? []).map((ch) => [ch.value, ch.label]) })),
      order: p.order ? { field: p.order.field, desc: p.order.desc } : null,
      sortable: p.sortable,
      page: p.page ?? null,
    });
  });
</script>

<Table
    frame={!ctx.inside || ctx.p.frame}
  columns={tableColumns}
  rows={ctx.collection?.rows ?? []}
  selected={ctx.selectedRow ?? null}
  design={ctx.design}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  resizable={p.resizable}
  sortable={p.sortable}
  order={p.order}
  remember={p.name ? `clarity-columns.${ctx.page.name.name}.${p.name}` : null}
  page={p.page}
  pageWords={{ first: ownWord(ctx.preview, "First page"), previous: ownWord(ctx.preview, "Previous page"), next: ownWord(ctx.preview, "Next page"), last: ownWord(ctx.preview, "Last page") }}
  ondesignwidth={ctx.design ? (field, width) => ctx.designWidth(field, width) : undefined}
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
