<script lang="ts">
  import { untrack } from "svelte";
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { displayValue, findEnum, findField, isNumeric } from "./model";
  import { pageDesigner } from "./pageDesigner.svelte";
  import { GAP, ROW_HEIGHT, allParts, boundField, fieldOf, findPart, helpId, partId, widgetInfo, type PageData } from "./pages";
  import type { Event as PartEvent } from "./generated/Event";
  import { useLive } from "./live";
  import { provideForm, useForm } from "./form";
  import { matchesShortcut } from "./shortcut";

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

  // In a running application: what the handlers set on this part.
  const live = useLive();
  const state = $derived(live && p.name ? live.parts[p.name] : undefined);
  const hidden = $derived(state?.visible === false);
  const disabled = $derived(state?.enabled === false);
  const caption = $derived(state?.caption !== undefined && state.caption !== null ? state.caption : p.caption);
  const selectedRow = $derived(live && p.name ? live.selected[p.name] : undefined);
  const readonly = $derived(state?.readonly ?? p.readonly);
  const error = $derived(state?.error ? state.error : null);

  // Labels and help texts belong to the input named in `for`.
  const target = $derived(findPart(page, p.for));
  const targetField = $derived(target ? fieldOf(data, target.props.bind) : undefined);
  const targetError = $derived(live && p.for ? live.parts[p.for]?.error || null : null);
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

  function submitOnEnter(e: KeyboardEvent) {
    if (e.key === "Enter" && !design && form?.submit()) e.preventDefault();
  }

  // A button's shortcut, and Enter and Escape for the default and cancel
  // buttons, work anywhere in the running window.
  function windowKey(e: KeyboardEvent) {
    if (design || !live || p.kind !== "button" || hidden || disabled || e.defaultPrevented) return;
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
    const inTextArea = e.target instanceof HTMLTextAreaElement;
    const onButton = e.target instanceof HTMLButtonElement;
    const hit =
      (p.shortcut !== null && matchesShortcut(e, p.shortcut)) ||
      (p.default && plain && e.key === "Enter" && !inTextArea && !onButton) ||
      (p.cancel && plain && e.key === "Escape");
    if (!hit) return;
    e.preventDefault();
    click();
  }

  // A bound input shows its field; an unbound text input keeps its own text,
  // which the running application sends with the next event.
  const shown = $derived(bound || !p.name ? text : (state?.text ?? ""));

  function setValue(next: string | boolean | number | null) {
    if (bound) bound.record[bound.field.name.name] = next;
  }

  function setText(next: string) {
    if (bound) setValue(next);
    else if (live && p.name && !design) live.setText(p.name, next);
  }

  /** Runs the handler wired to `event`, in a running application. */
  function fire(event: PartEvent) {
    const handler = p.handlers.find((h) => h.event === event)?.handler;
    if (live && handler && p.name && !design) live.event(p.name, event, handler);
  }

  function click() {
    if (design) return;
    if (p.handlers.some((h) => h.event === "click")) fire("click");
    else if (p.onClick) onnavigate(p.onClick);
  }

  function selectRow(index: number) {
    if (!live || !p.name || design) return;
    live.select(p.name, index);
    fire("select");
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

<svelte:window onkeydown={windowKey} />

<div
  class="widget {p.kind}"
  class:design
  class:selected={design && pageDesigner.selected === offset}
  class:lifted={pageDesigner.drag?.kind === "move" && pageDesigner.drag.item === offset}
  class:hidden
  class:disabled
  class:invalid={error !== null}
  data-widget={offset}
  title={error ?? p.tooltip ?? undefined}
  style:grid-column="{p.col} / span {p.width}"
  style:grid-row="{p.row} / span {p.height}"
  onpointerdown={pressMove}
  role="presentation"
>
  <div class="content" class:inert={design && !info.container}>
    {#if p.kind === "label"}
      <label class="label-text" for={p.for ? partId(p.for) : undefined}
        >{caption ?? ""}{#if targetField?.modifiers.required}<span class="required" title="Required">*</span>{/if}</label
      >
    {:else if p.kind === "help"}
      <span class="help-text" class:error={targetError !== null} id={p.for ? helpId(p.for) : undefined} aria-live="polite"
        >{targetError ?? caption ?? targetField?.modifiers.help ?? ""}</span
      >
    {:else if p.kind === "heading"}
      <h2 class="heading-text">{caption ?? ""}</h2>
    {:else if p.kind === "text"}
      <span class="text-value" {id}>{bound ? displayValue(preview, bound.field, value) : ""}</span>
    {:else if p.kind === "textbox" || p.kind === "password"}
      <input
        {id}
        type={p.kind === "password" ? "password" : bound?.field.type.kind === "email" ? "email" : "text"}
        value={shown}
        placeholder={p.placeholder ?? undefined}
        tabindex={p.tabOrder ?? undefined}
        aria-describedby={describedBy}
        aria-invalid={error !== null}
        {disabled}
        {readonly}
        oninput={(e) => setText(e.currentTarget.value)}
        onchange={() => fire("change")}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
        onkeydown={submitOnEnter}
      />
    {:else if p.kind === "memo"}
      <textarea
        {id}
        value={shown}
        placeholder={p.placeholder ?? undefined}
        tabindex={p.tabOrder ?? undefined}
        aria-describedby={describedBy}
        aria-invalid={error !== null}
        {disabled}
        {readonly}
        oninput={(e) => setText(e.currentTarget.value)}
        onchange={() => fire("change")}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
      ></textarea>
    {:else if p.kind === "number"}
      <input
        {id}
        class="numeric"
        inputmode="decimal"
        value={text}
        placeholder={p.placeholder ?? undefined}
        tabindex={p.tabOrder ?? undefined}
        aria-describedby={describedBy}
        aria-invalid={error !== null}
        {disabled}
        {readonly}
        oninput={(e) => setValue(e.currentTarget.value)}
        onchange={() => fire("change")}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
        onkeydown={submitOnEnter}
      />
    {:else if p.kind === "date"}
      <input
        {id}
        type={bound?.field.type.kind === "dateTime" ? "datetime-local" : "date"}
        value={text}
        tabindex={p.tabOrder ?? undefined}
        aria-describedby={describedBy}
        aria-invalid={error !== null}
        {disabled}
        {readonly}
        oninput={(e) => setValue(e.currentTarget.value === "" ? null : e.currentTarget.value)}
        onchange={() => fire("change")}
        onkeydown={submitOnEnter}
      />
    {:else if p.kind === "checkbox" || p.kind === "switch"}
      <label class={p.kind === "switch" ? "toggle" : "check"}>
        <input
          {id}
          type="checkbox"
          role={p.kind === "switch" ? "switch" : undefined}
          checked={value === true}
          tabindex={p.tabOrder ?? undefined}
          aria-describedby={describedBy}
          aria-invalid={error !== null}
          disabled={disabled || readonly}
          onchange={(e) => {
            setValue(e.currentTarget.checked);
            fire("change");
          }}
        />
        {#if p.kind === "switch"}<span class="track" aria-hidden="true"></span>{/if}
        {caption ?? ""}
      </label>
    {:else if p.kind === "radio"}
      <div class="choices {p.orientation ?? 'horizontal'}" role="radiogroup" aria-describedby={describedBy} aria-invalid={error !== null}>
        {#if bound?.field.type.kind === "enum"}
          {#each findEnum(preview, bound.field.type.name.name)?.variants ?? [] as v, i (v.name.name)}
            <label>
              <input
                id={i === 0 ? id : undefined}
                type="radio"
                name="radio-{offset}"
                value={v.name.name}
                checked={text === v.name.name}
                tabindex={p.tabOrder ?? undefined}
                disabled={disabled || readonly}
                onchange={() => {
                  setValue(v.name.name);
                  fire("change");
                }}
              />
              {v.label}
            </label>
          {/each}
        {/if}
      </div>
    {:else if p.kind === "dropdown"}
      <select
        {id}
        value={text}
        tabindex={p.tabOrder ?? undefined}
        aria-describedby={describedBy}
        aria-invalid={error !== null}
        disabled={disabled || readonly}
        onchange={(e) => {
          const chosen = e.currentTarget.value;
          // A reference holds the id of the record it points at.
          setValue(live && bound?.field.type.kind === "ref" ? (chosen === "" ? null : Number(chosen)) : chosen);
          fire("change");
        }}
      >
        <option value=""></option>
        {#if bound?.field.type.kind === "enum"}
          {#each findEnum(preview, bound.field.type.name.name)?.variants ?? [] as v (v.name.name)}
            <option value={v.name.name}>{v.label}</option>
          {/each}
        {:else if bound?.field.type.kind === "ref" && live}
          {#each live.refOptions(bound.field.type.entity.name) as option (option.id)}
            <option value={String(option.id)}>{option.text}</option>
          {/each}
        {:else if bound?.field.type.kind === "ref"}
          {#each preview.displays[bound.field.type.entity.name] ?? [] as option (option)}
            <option value={option}>{option}</option>
          {/each}
        {/if}
      </select>
    {:else if p.kind === "button"}
      <button {id} class="push" class:default={p.default} {disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={p.shortcut ?? undefined} onclick={click}
        >{caption ?? ""}</button
      >
    {:else if p.kind === "panel" || p.kind === "form"}
      <fieldset class={p.kind} class:bare={p.kind === "form" && !p.caption}>
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
              <tr class:chosen={selectedRow === i} onclick={() => selectRow(i)} ondblclick={() => fire("open")}>
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
          <li class:chosen={selectedRow === i} role="presentation" onclick={() => selectRow(i)} ondblclick={() => fire("open")}>{p.show.map((f) => String(row[f] ?? "")).join(" | ")}</li>
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
  .widget.hidden {
    visibility: hidden;
  }

  .widget.disabled {
    opacity: 0.55;
  }

  tr.chosen td,
  li.chosen {
    background: var(--accent-soft, rgb(80 140 255 / 0.18));
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

  .label-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .required {
    color: var(--error);
    margin-left: 2px;
  }

  .help-text {
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .help-text.error {
    color: var(--error);
  }

  .heading-text {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .text-value {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .widget.invalid :global(input),
  .widget.invalid :global(select),
  .widget.invalid :global(textarea) {
    border-color: var(--error);
  }

  textarea {
    width: 100%;
    height: 100%;
    resize: none;
    font: inherit;
  }

  .check,
  .toggle {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .track {
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: 9px;
    background: var(--border-strong);
    transition: background 0.15s;
  }

  .track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--panel);
    transition: transform 0.15s;
  }

  .toggle input:checked + .track {
    background: var(--accent);
  }

  .toggle input:checked + .track::after {
    transform: translateX(14px);
  }

  .toggle input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
  }

  .choices.vertical {
    flex-direction: column;
  }

  .choices label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .push.default {
    border-color: var(--accent);
  }

  /* A form groups its inputs; it shows a frame only when it has a caption,
     and without one its grid lines up with the grid around it. */
  fieldset.form.bare {
    border: none;
    padding: 0;
  }

  .widget.design > .content > fieldset.form.bare {
    outline: 1px dashed var(--border-strong);
  }

  .push {
    width: 100%;
    height: 100%;
  }

  .numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  /* Containers fill their area and lay out their own grid; only their own
     content, not the parts inside them. */
  .form > .content,
  .memo > .content,
  .panel > .content,
  .tabs > .content,
  .splitter > .content,
  .table > .content,
  .list > .content,
  .image > .content,
  .canvas > .content {
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
