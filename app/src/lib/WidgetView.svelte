<script lang="ts">
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { displayValue, findEnum, findField, isNumeric } from "./model";
  import { pageDesigner } from "./pageDesigner.svelte";
  import { GAP, ROW_HEIGHT, boundField, widgetInfo, type PageData } from "./pages";

  let {
    preview,
    page,
    data,
    design,
    widget,
    onnavigate,
  }: {
    preview: Preview;
    page: Page;
    data: PageData;
    design: boolean;
    widget: Widget;
    onnavigate: (target: string) => void;
  } = $props();

  const p = $derived(widget.props);
  const offset = $derived(widget.span.start);
  const info = $derived(widgetInfo(preview, p.kind));
  const bound = $derived(boundField(data, p.bind));
  const value = $derived(bound ? bound.record[bound.field.name.name] : null);
  const text = $derived(value === null || value === undefined ? "" : String(value));
  const collection = $derived(p.bind && !p.bind.includes(".") ? data[p.bind] : undefined);
  const activeTab = $derived(Math.min(pageDesigner.activeTabs[offset] ?? 0, Math.max(0, widget.pages.length - 1)));

  function setValue(next: string | boolean) {
    if (bound) bound.record[bound.field.name.name] = next;
  }

  function pressMove(e: PointerEvent) {
    if (!design) return;
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const cellWidth = (rect.width - GAP * (p.width - 1)) / p.width;
    const grab = {
      dc: Math.min(p.width - 1, Math.floor((e.clientX - rect.left) / (cellWidth + GAP))),
      dr: Math.min(p.height - 1, Math.floor((e.clientY - rect.top) / (ROW_HEIGHT + GAP))),
    };
    const from = Number(el.parentElement?.closest<HTMLElement>("[data-grid]")?.dataset.grid);
    pageDesigner.press(e, { kind: "move", item: offset, from, props: p, grab, label: info.name }, () => {
      pageDesigner.selected = offset;
    });
  }

  function pressResize(e: PointerEvent) {
    pageDesigner.press(e, { kind: "resize", item: offset, props: p, label: `${p.width} x ${p.height}` }, () => {
      pageDesigner.selected = offset;
    });
  }

  function pressTab(e: PointerEvent, index: number, tabOffset: number) {
    e.stopPropagation();
    pageDesigner.activeTabs[offset] = index;
    if (design) pageDesigner.selected = tabOffset;
  }

  function columnLabel(name: string): string {
    return (collection && findField(collection.entity, name)?.label) ?? name;
  }
</script>

<div
  class="widget {p.kind}"
  class:design
  class:selected={design && pageDesigner.selected === offset}
  class:lifted={pageDesigner.drag?.kind === "move" && pageDesigner.drag.item === offset}
  data-widget={offset}
  style:grid-column="{p.col} / span {p.width}"
  style:grid-row="{p.row} / span {p.height}"
  onpointerdown={pressMove}
  role="presentation"
>
  <div class="content" class:inert={design && !info.container}>
    {#if p.kind === "label"}
      <span class="label-text">{p.caption ?? (bound ? text : "")}{#if p.caption && bound}: <strong>{text}</strong>{/if}</span>
    {:else if p.kind === "textbox"}
      <input type={bound?.field.type.kind === "email" ? "email" : "text"} value={text} placeholder={p.caption ?? ""} oninput={(e) => setValue(e.currentTarget.value)} />
    {:else if p.kind === "number"}
      <input class="numeric" inputmode="decimal" value={text} oninput={(e) => setValue(e.currentTarget.value)} />
    {:else if p.kind === "date"}
      <input type={bound?.field.type.kind === "dateTime" ? "datetime-local" : "date"} value={text} oninput={(e) => setValue(e.currentTarget.value)} />
    {:else if p.kind === "checkbox"}
      <label class="check">
        <input type="checkbox" checked={value === true} onchange={(e) => setValue(e.currentTarget.checked)} />
        {p.caption ?? bound?.field.label ?? ""}
      </label>
    {:else if p.kind === "dropdown"}
      <select value={text} onchange={(e) => setValue(e.currentTarget.value)}>
        <option value=""></option>
        {#if bound?.field.type.kind === "enum"}
          {#each findEnum(preview, bound.field.type.name.name)?.variants ?? [] as v (v.name.name)}
            <option value={v.name.name}>{v.label}</option>
          {/each}
        {:else if bound?.field.type.kind === "ref"}
          {#each preview.displays[bound.field.type.entity.name] ?? [] as option (option)}
            <option value={option}>{option}</option>
          {/each}
        {/if}
      </select>
    {:else if p.kind === "button"}
      <button class="push" onclick={() => !design && p.onClick && onnavigate(p.onClick)}>{p.caption ?? ""}</button>
    {:else if p.kind === "panel"}
      <fieldset>
        {#if p.caption}<legend>{p.caption}</legend>{/if}
        <GridBox {preview} {page} {data} {design} widgets={widget.children} columns={p.width} container={offset} {onnavigate} />
      </fieldset>
    {:else if p.kind === "tabs"}
      <div class="tabs">
        <div class="tab-strip">
          {#each widget.pages as tab, index (tab.span.start)}
            <button
              class:active={index === activeTab}
              class:selected={design && pageDesigner.selected === tab.span.start}
              data-tab-switch="{offset}:{index}"
              onpointerdown={(e) => pressTab(e, index, tab.span.start)}>{tab.title}</button
            >
          {/each}
        </div>
        {#if widget.pages[activeTab]}
          {@const tab = widget.pages[activeTab]}
          <div class="tab-page">
            <GridBox {preview} {page} {data} {design} widgets={tab.children} columns={p.width} container={tab.span.start} {onnavigate} />
          </div>
        {/if}
      </div>
    {:else if p.kind === "splitter"}
      <div class="splitter {p.orientation ?? 'horizontal'}">
        {#each widget.pages as pane, index (pane.span.start)}
          {#if index > 0}<div class="divider"></div>{/if}
          <div class="pane">
            <GridBox {preview} {page} {data} {design} widgets={pane.children} columns={p.width} container={pane.span.start} {onnavigate} />
          </div>
        {/each}
      </div>
    {:else if p.kind === "table"}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each p.columns as column (column)}<th>{columnLabel(column)}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each collection?.rows ?? [] as row, i (i)}
              <tr>
                {#each p.columns as column (column)}
                  {@const field = collection ? findField(collection.entity, column) : undefined}
                  <td class:numeric={field ? isNumeric(field) : false}>{field ? displayValue(preview, field, row[column]) : ""}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
        {#if p.columns.length === 0}<div class="empty">Choose columns in the properties</div>{/if}
      </div>
    {:else if p.kind === "list"}
      <ul class="list">
        {#each collection?.rows ?? [] as row, i (i)}
          <li>{p.show.map((f) => String(row[f] ?? "")).join(" | ")}</li>
        {/each}
      </ul>
    {:else if p.kind === "image"}
      <div class="placeholder image">
        <span>Image</span>
        <code>{p.source ?? "no source"}</code>
      </div>
    {:else if p.kind === "canvas"}
      <div class="placeholder canvas">
        <span>Canvas ({p.surface ?? "svg"})</span>
        <code>{p.renderer ?? "no renderer"}</code>
      </div>
    {/if}
  </div>

  {#if design}
    <span class="tag">{info.name}</span>
    {#if pageDesigner.selected === offset}
      <div class="resize" onpointerdown={pressResize} role="presentation" title="Drag to resize"></div>
    {/if}
  {/if}
</div>

<style>
  .widget {
    position: relative;
    min-width: 0;
    min-height: 0;
    display: flex;
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .content > :global(input),
  .content > :global(select) {
    width: 100%;
  }

  .inert {
    pointer-events: none;
  }

  .widget.design {
    outline: 1px dashed transparent;
    outline-offset: 2px;
    cursor: grab;
    touch-action: none;
  }

  .widget.design:hover {
    outline-color: var(--border-strong);
  }

  .widget.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    z-index: 2;
  }

  .widget.lifted {
    opacity: 0.3;
  }

  .tag {
    position: absolute;
    top: -9px;
    right: 4px;
    font-size: 9px;
    line-height: 1;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    color: var(--text-muted);
    opacity: 0;
    pointer-events: none;
  }

  .widget.design:hover > .tag,
  .widget.selected > .tag {
    opacity: 1;
  }

  .resize {
    position: absolute;
    right: -6px;
    bottom: -6px;
    width: 12px;
    height: 12px;
    border-radius: 2px;
    background: var(--accent);
    border: 2px solid var(--panel);
    cursor: nwse-resize;
    z-index: 3;
  }

  .label-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .check {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .push {
    width: 100%;
    height: 100%;
  }

  .numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  /* Containers fill their area and lay out their own grid. */
  .panel .content,
  .tabs .content,
  .splitter .content,
  .table .content,
  .list .content,
  .image .content,
  .canvas .content {
    align-items: stretch;
  }

  fieldset {
    flex: 1;
    min-width: 0;
    margin: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 8px;
  }

  legend {
    padding: 0 6px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .tabs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .tab-strip {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border-strong);
  }

  .tab-strip button {
    border-radius: var(--radius) var(--radius) 0 0;
    border-bottom: none;
    margin-bottom: -1px;
    background: var(--panel-alt);
    color: var(--text-muted);
  }

  .tab-strip button.active {
    background: var(--panel);
    color: var(--text);
    border-bottom: 1px solid var(--panel);
  }

  .tab-strip button.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tab-page {
    flex: 1;
    border: 1px solid var(--border-strong);
    border-top: none;
    border-radius: 0 0 var(--radius) var(--radius);
    padding: 8px;
    overflow: hidden;
  }

  .splitter {
    flex: 1;
    display: flex;
    min-width: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .splitter.vertical {
    flex-direction: column;
  }

  .pane {
    flex: 1;
    min-width: 0;
    padding: 8px;
    overflow: hidden;
  }

  .divider {
    flex: none;
    background: var(--border-strong);
    width: 4px;
  }

  .splitter.vertical .divider {
    width: auto;
    height: 4px;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    position: sticky;
    top: 0;
    background: var(--panel-alt);
    text-align: left;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-strong);
  }

  td {
    padding: 3px 8px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }

  .empty {
    padding: 8px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .list {
    flex: 1;
    margin: 0;
    padding: 4px;
    list-style: none;
    overflow: auto;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font-size: 13px;
  }

  .list li {
    padding: 3px 6px;
    border-bottom: 1px solid var(--border);
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--text-muted);
    font-size: 12px;
  }

  .placeholder code {
    font-family: var(--mono);
    font-size: 11px;
  }

  .placeholder.canvas {
    background-image:
      linear-gradient(45deg, var(--panel-alt) 25%, transparent 25%),
      linear-gradient(-45deg, var(--panel-alt) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, var(--panel-alt) 75%),
      linear-gradient(-45deg, transparent 75%, var(--panel-alt) 75%);
    background-size: 16px 16px;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  }
</style>
