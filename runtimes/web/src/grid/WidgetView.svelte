<script lang="ts">
  // A part on a grid: where it stands, what the designer does with it, its
  // context menu, and the one interface (PartContext) its kind's module draws
  // it from (DESIGN 7.8, G6a). How each kind looks and behaves is its module's,
  // in the screen package: @clarity/ui/kinds.
  import { untrack } from "svelte";
  import PartMenu from "../parts/PartMenu.svelte";
  import type { Command } from "../generated/Command.ts";
  import { itemClicks, itemShown } from "../screen/overlays.ts";
  import { previewPopovers } from "../screen/previewPopovers.svelte.ts";
  import GridBox from "./GridBox.svelte";
  import type { Page } from "../generated/Page.ts";
  import type { Preview } from "../generated/Preview.ts";
  import type { Widget } from "../generated/Widget.ts";
  import type { JsonValue } from "../generated/serde_json/JsonValue.ts";
  import { shownText, themeOf, tr } from "../screen/model.ts";
  import { styleClass } from "../screen/theme.ts";
  import { useDesigner } from "../screen/designer.svelte.ts";
  import { GAP, ROW_HEIGHT, widgetGrows, allParts, rowAt, boundField, fieldOf, findPart, helpId, partId, widgetInfo, withProps, type PageData } from "../screen/pages.ts";
  import type { Event as PartEvent } from "../generated/Event.ts";
  import { useLive } from "../screen/live.ts";
  import { provideForm, useForm } from "../screen/form.ts";
  import { wiredCall } from "../screen/calls.ts";
  import type { GridArgs, PartContext } from "../kinds/context.ts";
  import { kindModule } from "../kinds/index.ts";

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
    onnavigate: (target: string, newWindow?: boolean) => void;
  } = $props();

  // The page designer around the grid in the IDE; the idle one in a running application.
  const pageDesigner = useDesigner();

  const p = $derived(widget.props);
  const offset = $derived(widget.span.start);
  const info = $derived(widgetInfo(preview, p.kind));
  const bound = $derived(boundField(data, p.bind));
  const value = $derived(bound ? bound.record[bound.field.name.name] : null);
  const text = $derived(value === null || value === undefined ? "" : String(value));
  const collection = $derived(p.bind && !p.bind.includes(".") ? data[p.bind] : undefined);
  const activeTab = $derived(Math.min(pageDesigner.activeTabs[offset] ?? 0, Math.max(0, widget.pages.length - 1)));

  // In a running application: what the handlers set on this part.
  const live = useLive();
  const partState = $derived(live && p.name ? live.parts[p.name] : undefined);
  const hidden = $derived(partState?.visible === false);
  const disabled = $derived(partState?.enabled === false);
  // In the designer a text part without text of its own (code fills it in
  // when the window runs) shows its name, so it can be seen and picked.
  const unset = $derived(design && !p.caption ? (p.name ?? info.name) : null);
  const caption = $derived(partState?.caption !== undefined && partState.caption !== null ? shownText(preview, partState.caption) : p.caption !== null ? tr(preview, p.caption) : null);
  const selectedRow = $derived(live && p.name ? live.selected[p.name] : undefined);
  const readonly = $derived(partState?.readonly ?? p.readonly);
  const error = $derived(partState?.error ? shownText(preview, partState.error) || null : null);
  // The class of the part's style in the project's theme: on a button the button
  // itself, on other parts their content.
  const partClass = $derived(styleClass(themeOf(preview), p.style));

  // Labels and help texts belong to the input named in `for`.
  const target = $derived(findPart(page, p.for));
  const targetField = $derived(target ? fieldOf(data, target.props.bind) : undefined);
  const targetError = $derived.by(() => {
    const error = live && p.for ? live.parts[p.for]?.error : null;
    return error ? shownText(preview, error) || null : null;
  });
  const id = $derived(p.name ? partId(p.name) : undefined);
  const describedBy = $derived(
    p.name && allParts(page).some((w) => w.props.kind === "help" && w.props.for === p.name) ? helpId(p.name) : undefined,
  );

  // Enter in an input of a form runs the form's `on submit`. Context is set
  // once, when the part is created, so its kind at that moment decides.
  const form = useForm();
  if (untrack(() => widget.props.kind) === "form") {
    provideForm({
      submit() {
        if (!p.handlers.some((h) => h.event === "submit")) return false;
        fire("submit");
        return true;
      },
    });
  }

  /** Enter in an input of a form: runs the form's `on submit`; false when there is none. */
  function submit(): boolean {
    return !design && !!form?.submit();
  }

  // A bound input shows its field; an unbound text input keeps its own text,
  // which the running application sends with the next event.
  const shown = $derived(bound || !p.name ? text : (partState?.text ?? ""));

  function setValue(next: JsonValue) {
    if (!bound) return;
    // An edit for undo (DESIGN 5.29): the path, what it was and what it is now.
    if (live && !design && p.name && p.bind) live.edited?.(p.name);
    bound.record[bound.field.name.name] = next;
    // A setting is kept at once, and switches the language or theme it names.
    if (live && !design && p.bind?.split(".")[0] === "settings") live.keepSettings();
  }

  function setText(next: string) {
    if (bound) setValue(next);
    else if (live && p.name && !design) live.setText(p.name, next);
  }

  /** Runs the handler wired to `event`, in a running application; false when it refused. */
  function fire(event: PartEvent): Promise<boolean> {
    // The handler as written, with the values it runs with: `digit(7)`.
    const call = wiredCall(p.handlers, event);
    if (live && call !== null && !design) return live.event(p.name ?? "", event, call);
    return Promise.resolve(true);
  }

  function click() {
    if (design) return;
    if (p.handlers.some((h) => h.event === "click")) fire("click");
    else if (p.onClick && findPart(page, p.onClick)?.props.kind === "popover") openPopover(p.onClick);
    else if (p.onClick) onnavigate(p.onClick, p.newWindow);
  }

  /** Opens a popover of the window: live in its part state, in a preview here. */
  function openPopover(name: string) {
    if (live) live.setVisible(name, true);
    else previewPopovers[name] = true;
  }

  // The part's context menu, open at the pointer.
  let menuAt = $state<{ x: number; y: number } | null>(null);

  function openMenu(e: MouseEvent) {
    // A split button's menu opens from its arrow, not a right click.
    if (design || !widget.menu || disabled || p.split) return;
    e.preventDefault();
    e.stopPropagation();
    menuAt = { x: e.clientX, y: e.clientY };
  }

  /** A context menu's item: its handler, or the window or popover it opens. */
  function runItem(item: Command) {
    if (item.props.handler && live) live.event(item.props.name ?? "", "click", item.props.handler);
    else if (item.props.onClick && findPart(page, item.props.onClick)?.props.kind === "popover") openPopover(item.props.onClick);
    else if (item.props.onClick) onnavigate(item.props.onClick, item.props.newWindow);
  }

  /** Chooses a row; `select` runs when it is another row than before, as on every stack. */
  function selectRow(index: number) {
    if (!live || !p.name || design) return;
    const other = selectedRow !== index;
    live.select(p.name, index);
    if (other) fire("select");
  }

  // A part that does not grow keeps the height of its rows at their least,
  // at the top, when a part beside it makes them grow (as on every stack).
  const keepsHeight = $derived(!widgetGrows(preview, widget));

  /** A column's width dragged in the designer: written into the part, 0 taking it away. */
  function designWidth(field: string, width: number) {
    const widths = { ...p.widths };
    if (width > 0) widths[field] = width;
    else delete widths[field];
    pageDesigner.edit({ op: "updateWidget", item: widget.span.start, container: null, props: withProps(p, { widths }) });
  }

  function pressMove(e: PointerEvent) {
    // A label of a form's label column is the form's: the designer leaves it alone.
    if (!design || p.made) return;
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const cellWidth = (rect.width - GAP * (p.width - 1)) / p.width;
    // The row grabbed, counted in the grid's own rows: some are taller than others.
    const grid = el.parentElement?.closest<HTMLElement>("[data-grid]");
    const gridRect = grid?.getBoundingClientRect();
    const row = grid && gridRect ? rowAt(grid, e.clientY - gridRect.top, gridRect.height) : p.row;
    const grab = {
      dc: Math.min(p.width - 1, Math.floor((e.clientX - rect.left) / (cellWidth + GAP))),
      dr: Math.min(p.height - 1, Math.max(0, row - p.row)),
    };
    const from = Number(grid?.dataset.grid);
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

  // What the part's module draws it from: read through getters, so it stays live.
  const ctx: PartContext = {
    get preview() { return preview; },
    get page() { return page; },
    get data() { return data; },
    get widget() { return widget; },
    get p() { return p; },
    get info() { return info; },
    get offset() { return offset; },
    get design() { return design; },
    get bound() { return bound; },
    get value() { return value; },
    get shown() { return shown; },
    get collection() { return collection; },
    get caption() { return caption; },
    get unset() { return unset; },
    get error() { return error; },
    get partClass() { return partClass ?? null; },
    get id() { return id; },
    get describedBy() { return describedBy; },
    get targetField() { return targetField; },
    get targetError() { return targetError; },
    get live() { return live; },
    get partState() { return partState; },
    get disabled() { return disabled; },
    get readonly() { return readonly; },
    get selectedRow() { return selectedRow; },
    setValue,
    setText,
    fire,
    submit,
    click,
    selectRow,
    openMenu: (x, y) => (menuAt = { x, y }),
    onnavigate: (target, newWindow) => onnavigate(target, newWindow),
    get activeTab() { return activeTab; },
    isSelected: (at) => pageDesigner.selected === at,
    pressTab,
    designWidth,
    get grid() { return grid; },
  };

  const Kind = $derived(kindModule(ctx));
</script>

{#snippet grid(args: GridArgs)}
  <GridBox {preview} {page} {data} {design} widgets={args.widgets} columns={args.columns} fixedRows={args.fixedRows} joined={args.joined} container={args.container} {onnavigate} />
{/snippet}

<div
  class="widget {p.kind}"
  class:design
  class:selected={design && pageDesigner.selected === offset && !p.made}
  class:lifted={pageDesigner.drag?.kind === "move" && pageDesigner.drag.item === offset}
  class:hidden
  class:disabled
  class:invalid={error !== null}
  data-widget={offset}
  data-part={p.name}
  oncontextmenu={openMenu}
  title={error ?? (p.tooltip ? tr(preview, p.tooltip) : undefined)}
  style:grid-column="{p.col} / span {p.width}"
  style:grid-row="{p.row} / span {p.height}"
  style:align-self={keepsHeight ? "start" : undefined}
  style:height={keepsHeight ? `calc(${p.height} * ${ROW_HEIGHT}px + ${p.height - 1} * var(--grid-gap, ${GAP}px))` : undefined}
  onpointerdown={pressMove}
  ondblclick={(e) => {
    if (!design || p.made) return;
    e.stopPropagation();
    pageDesigner.openPartCode(offset);
  }}
  role="presentation"
>
  <div class="content {p.kind === 'button' ? '' : (partClass ?? '')}" class:inert={design && !info.container}>
    {#if Kind}<Kind {ctx} />{/if}
  </div>

  {#if design}
    <span class="tag">{info.name}</span>
    {#if pageDesigner.selected === offset && !p.made}
      <div class="resize" onpointerdown={pressResize} role="presentation" title="Drag to resize"></div>
    {/if}
  {/if}
</div>

{#if menuAt && widget.menu}
  <PartMenu
    items={widget.menu.items}
    icons={preview.icons}
    x={menuAt.x}
    y={menuAt.y}
    caption={(text) => tr(preview, text)}
    shows={(name) => !live || !name || itemShown(live.parts[name])}
    enabled={(name) => !live || !name || itemClicks(live.parts[name])}
    onrun={runItem}
    onclose={() => (menuAt = null)}
  />
{/if}

<style>
  .widget.hidden {
    visibility: hidden;
  }

  .widget.disabled {
    opacity: 0.55;
  }

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

  .widget.invalid :global(input),
  .widget.invalid :global(select),
  .widget.invalid :global(textarea) {
    border-color: var(--error);
  }


  /* Containers fill their area and lay out their own grid; only their own
     content, not the parts inside them. */
  .form > .content,
  .memo > .content,
  .panel > .content,
  .tabs > .content,
  .accordion > .content,
  .scroll > .content,
  .steps > .content,
  .splitter > .content,
  .table > .content,
  .tree > .content,
  .treeTable > .content,
  .chart > .content,
  .list > .content,
  .image > .content,
  .canvas > .content {
    align-items: stretch;
  }

  /* A paged table's bar goes below it. */
  .table > .content {
    flex-direction: column;
  }
</style>
