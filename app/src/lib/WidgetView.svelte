<script lang="ts">
  import { untrack } from "svelte";
  import DateTimeInput from "./ui/DateTimeInput.svelte";
  import Table, { type CellKind, type TableColumn } from "./ui/Table.svelte";
  import CheckInput from "./ui/CheckInput.svelte";
  import ChoiceInput from "./ui/ChoiceInput.svelte";
  import DateInput from "./ui/DateInput.svelte";
  import NumberInput from "./ui/NumberInput.svelte";
  import RadioInput from "./ui/RadioInput.svelte";
  import TextInput from "./ui/TextInput.svelte";
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import type { JsonValue } from "./generated/serde_json/JsonValue";
  import { displayValue, fieldFormat, findEnum, findField, formatsOf, isNumeric, shownText, themeOf, tr } from "./model";
  import { styleClass } from "./theme";
  import { numberEditing } from "./format";
  import { pageDesigner } from "./pageDesigner.svelte";
  import { GAP, allParts, rowAt, boundField, fieldOf, findPart, helpId, partId, widgetInfo, type PageData } from "./pages";
  import type { Event as PartEvent } from "./generated/Event";
  import { useLive } from "./live";
  import { provideForm, useForm } from "./form";
  import { matchesShortcut } from "./shortcut";
  import { pictureUrl } from "./pictures.svelte";
  import { isStored, linkedFile, pictureFromFile, pictureUrl as recordPictureUrl } from "./recordPictures";

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
  const shown = $derived(bound || !p.name ? text : (partState?.text ?? ""));

  function setValue(next: JsonValue) {
    if (bound) bound.record[bound.field.name.name] = next;
  }

  // A picture field: its stored picture comes on its own; a new one is
  // chosen from a file and kept in the field until the record is saved.
  const boundEntity = $derived(p.bind ? data[p.bind.split(".")[0]]?.entity.name.name : undefined);
  const canLoadPicture = $derived(!!live && !design && !readonly && !disabled);
  let fileInput = $state<HTMLInputElement | null>(null);
  let pictureProblem = $state(false);

  function forgetFetched() {
    if (live && bound && boundEntity) live.forgetPicture(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name);
  }

  async function pictureChosen(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = "";
    if (!file) return;
    const picture = pictureFromFile(new Uint8Array(await file.arrayBuffer()));
    pictureProblem = picture === null;
    if (!picture) return;
    forgetFetched();
    setValue(picture);
  }

  function removePicture() {
    pictureProblem = false;
    forgetFetched();
    setValue(null);
  }

  // A number input shows its value in the person's format and reads what is
  // typed the same way; unbound, or without a format, it keeps the text.
  const numberFormat = $derived(bound ? fieldFormat(bound.field) : null);
  const numbers = $derived(
    numberFormat ? numberEditing(numberFormat, formatsOf(preview)) : { text: (v: unknown) => String(v ?? ""), read: (typed: string) => typed },
  );

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
    else if (p.onClick) onnavigate(p.onClick, p.newWindow);
  }

  /** Chooses a row; `select` runs when it is another row than before, as on every stack. */
  function selectRow(index: number) {
    if (!live || !p.name || design) return;
    const other = selectedRow !== index;
    live.select(p.name, index);
    if (other) fire("select");
  }

  /** The table's columns for the library part: how each field shows and is edited. */
  const tableColumns = $derived.by((): TableColumn[] => {
    if (p.kind !== "table" || !collection) return [];
    const entity = collection.entity;
    return p.columns.map((name) => {
      const field = findField(entity, name);
      const label = field ? tr(preview, field.label) : name;
      if (!field) return { field: name, label, kind: "text", edit: false };
      const kind = cellKind(field.type.kind);
      const format = fieldFormat(field);
      return {
        field: name,
        label,
        kind,
        edit: p.edit.includes(name),
        switch: p.switches.includes(name),
        shown: (value) => displayValue(preview, field, value),
        choices: kind === "choice" ? choicesOf(field) : undefined,
        number: kind === "number" && format ? numberEditing(format, formatsOf(preview)) : undefined,
      };
    });
  });

  /** What a cell of a field's type holds. */
  function cellKind(kind: string): CellKind {
    switch (kind) {
      case "integer":
      case "decimal":
      case "money":
        return "number";
      case "bool":
        return "bool";
      case "enum":
      case "ref":
        return "choice";
      case "date":
        return "date";
      case "dateTime":
        return "moment";
      default:
        return "text";
    }
  }

  /** An enum's values (never empty), or the records a reference can point at after an empty choice. */
  function choicesOf(field: NonNullable<ReturnType<typeof findField>>): TableColumn["choices"] {
    const type = field.type;
    if (type.kind === "enum") return (findEnum(preview, type.name.name)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(preview, v.label) }));
    if (type.kind !== "ref") return [];
    // Running, a reference holds the id of its record; in the designer, the sample's text.
    const records = live
      ? live.refOptions(type.entity.name).map((o) => ({ value: o.id, label: o.text }))
      : (preview.displays[type.entity.name] ?? []).map((text) => ({ value: text, label: text }));
    return [{ value: null, label: "" }, ...records];
  }

  function pressMove(e: PointerEvent) {
    if (!design) return;
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
  title={error ?? (p.tooltip ? tr(preview, p.tooltip) : undefined)}
  style:grid-column="{p.col} / span {p.width}"
  style:grid-row="{p.row} / span {p.height}"
  onpointerdown={pressMove}
  role="presentation"
>
  <div class="content {p.kind === 'button' ? '' : (partClass ?? '')}" class:inert={design && !info.container}>
    {#if p.kind === "label"}
      <label class="label-text" for={p.for ? partId(p.for) : undefined}
        >{#if unset && !caption}<span class="unset">{unset}</span>{/if}{caption ?? ""}{#if targetField?.modifiers.required}<span class="required" title="Required">*</span>{/if}</label
      >
    {:else if p.kind === "help"}
      <span class="help-text" class:error={targetError !== null} id={p.for ? helpId(p.for) : undefined} aria-live="polite"
        >{targetError ?? caption ?? (targetField?.modifiers.help ? tr(preview, targetField.modifiers.help) : "")}</span
      >
    {:else if p.kind === "heading"}
      <h2 class="heading-text">{#if unset && !caption}<span class="unset">{unset}</span>{/if}{caption ?? ""}</h2>
    {:else if p.kind === "text"}
      <span class="text-value" {id}>{bound ? displayValue(preview, bound.field, value) : ""}</span>
    {:else if p.kind === "textbox" || p.kind === "password" || p.kind === "memo"}
      <TextInput
        {id}
        kind={p.kind === "password" ? "password" : p.kind === "memo" ? "memo" : bound?.field.type.kind === "email" ? "email" : "text"}
        value={shown}
        placeholder={p.placeholder ? tr(preview, p.placeholder) : undefined}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onchange={(next) => {
          setText(next);
          fire("change");
        }}
        onsubmit={submit}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
      />
    {:else if p.kind === "number"}
      <NumberInput
        {id}
        value={bound ? value : shown}
        text={numbers.text}
        read={numbers.read}
        placeholder={p.placeholder ? tr(preview, p.placeholder) : undefined}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onchange={(next) => {
          if (bound) setValue(next);
          else setText(String(next));
          fire("change");
        }}
        onsubmit={submit}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
      />
    {:else if p.kind === "date" && bound?.field.type.kind === "dateTime"}
      <!-- A moment: a day and a 24-hour time, as on every stack. -->
      <DateTimeInput
        {id}
        {value}
        {disabled}
        {readonly}
        invalid={error !== null}
        {describedBy}
        onkeydown={(e) => {
          if (e.key === "Enter" && submit()) e.preventDefault();
        }}
        onchange={(next) => {
          setValue(next);
          fire("change");
        }}
      />
    {:else if p.kind === "date"}
      <DateInput
        {id}
        {value}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onchange={(next) => {
          setValue(next);
          fire("change");
        }}
        onsubmit={submit}
      />
    {:else if p.kind === "checkbox" || p.kind === "switch"}
      <CheckInput
        {id}
        switch={p.kind === "switch"}
        caption={caption ?? undefined}
        checked={bound ? value === true : partState?.checked === true}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        disabled={disabled || readonly}
        onchange={(next) => {
          if (bound) setValue(next);
          else if (live && p.name && !design) live.setChecked(p.name, next);
          fire("change");
        }}
      />
    {:else if p.kind === "radio"}
      <RadioInput
        {id}
        name="radio-{offset}"
        {value}
        choices={bound ? (choicesOf(bound.field) ?? []) : []}
        vertical={p.orientation === "vertical"}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        disabled={disabled || readonly}
        onchange={(next) => {
          setValue(next);
          fire("change");
        }}
      />
    {:else if p.kind === "dropdown"}
      <ChoiceInput
        {id}
        {value}
        choices={bound ? (choicesOf(bound.field) ?? []) : [{ value: null, label: "" }]}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        disabled={disabled || readonly}
        onchange={(next) => {
          setValue(next);
          fire("change");
        }}
      />
    {:else if p.kind === "button"}
      <button {id} class="push {partClass ?? ''}" class:default={p.default} {disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={p.shortcut ?? undefined} onclick={click}
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
              onpointerdown={(e) => pressTab(e, index, tab.span.start)}>{tab.title ? tr(preview, tab.title) : tab.title}</button
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
      <Table
        columns={tableColumns}
        rows={collection?.rows ?? []}
        selected={selectedRow ?? null}
        {design}
        {disabled}
        {readonly}
        onselect={(row) => selectRow(row)}
        onopen={() => fire("open")}
        onedit={() => fire("change")}
      />
    {:else if p.kind === "list"}
      <ul class="list">
        {#each collection?.rows ?? [] as row, i (i)}
          <li class:chosen={selectedRow === i} role="presentation" onclick={() => selectRow(i)} ondblclick={() => fire("open")}>{p.show.map((f) => String(row[f] ?? "")).join(" | ")}</li>
        {/each}
      </ul>
    {:else if p.kind === "image" && p.bind}
      {@const linked = linkedFile(value)}
      {@const fetched = !live || !bound
        ? undefined
        : linked && bound.field.modifiers.store
          ? live.storeFile(bound.field.modifiers.store, linked)
          : boundEntity && isStored(value)
            ? live.picture(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name)
            : undefined}
      {@const shownPicture = recordPictureUrl(value, fetched)}
      <div class="picture-field">
        {#if shownPicture}
          <img class="picture" src={shownPicture} alt={caption ?? ""} />
        {:else}
          <div class="placeholder image" class:problem={pictureProblem}>
            <span>{tr(preview, "No picture")}</span>
          </div>
        {/if}
        {#if canLoadPicture}
          <div class="picture-actions">
            <button type="button" onclick={() => fileInput?.click()}>{tr(preview, "Load a picture")}</button>
            {#if value}
              <button type="button" onclick={removePicture}>{tr(preview, "Remove the picture")}</button>
            {/if}
          </div>
          <input bind:this={fileInput} type="file" accept="image/png,image/jpeg,image/gif,image/bmp,image/webp" hidden onchange={pictureChosen} />
        {/if}
      </div>
    {:else if p.kind === "image"}
      {@const picture = p.source ? pictureUrl(p.source) : null}
      {#if picture}
        <img class="picture" src={picture} alt={p.tooltip ? tr(preview, p.tooltip) : ""} />
      {:else}
        <div class="placeholder image">
          <span>Image</span>
          <code>{p.source ?? "no source"}</code>
        </div>
      {/if}
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

  li.chosen {
    background: var(--selection);
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
    font-size: var(--font-size-small);
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
    font-family: var(--font-heading);
    font-size: var(--font-size-heading);
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

  .picture {
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: contain;
  }

  .picture-field {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
  }

  /* The buttons show when the pointer or the keyboard is on the picture. */
  .picture-actions {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    justify-content: center;
    gap: 4px;
    padding: 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .picture-field:hover .picture-actions,
  .picture-field:focus-within .picture-actions {
    opacity: 1;
  }

  .placeholder.problem {
    outline: 2px solid var(--error);
  }

  fieldset {
    flex: 1;
    min-width: 0;
    margin: 0;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    padding: calc(var(--padding) * 2);
  }

  legend {
    padding: 0 6px;
    font-size: var(--font-size-small);
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
    border-bottom: var(--border-width) solid var(--border-strong);
  }

  .tab-strip button {
    border-radius: var(--tab-radius, var(--radius-button)) var(--tab-radius, var(--radius-button)) 0 0;
    border-bottom: none;
    margin-bottom: -1px;
    background: var(--tab-bg);
    color: var(--tab-text);
    border-color: var(--tab-border);
    font-weight: var(--tab-weight, inherit);
  }

  .tab-strip button:hover:not(.active) {
    background: var(--tab-hover-bg, var(--tab-bg));
    color: var(--tab-hover-text, var(--tab-text));
  }

  .tab-strip button.active {
    background: var(--panel);
    color: var(--text);
    border-bottom: var(--border-width) solid var(--panel);
  }

  .tab-strip button.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tab-page {
    flex: 1;
    border: var(--border-width) solid var(--border-strong);
    border-top: none;
    border-radius: 0 0 var(--radius-panel) var(--radius-panel);
    padding: calc(var(--padding) * 2);
    overflow: hidden;
  }

  .splitter {
    flex: 1;
    display: flex;
    min-width: 0;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    overflow: hidden;
  }

  .splitter.vertical {
    flex-direction: column;
  }

  .pane {
    flex: 1;
    min-width: 0;
    padding: calc(var(--padding) * 2);
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

  .list {
    flex: 1;
    margin: 0;
    padding: 4px;
    list-style: none;
    overflow: auto;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    font-size: var(--font-size);
  }

  .list li {
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 1.5);
    border-bottom: var(--border-width) solid var(--border);
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: var(--border-width) dashed var(--border-strong);
    border-radius: var(--radius-panel);
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .placeholder code {
    font-family: var(--mono);
    font-size: var(--font-size-small);
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

  .unset {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }
</style>
