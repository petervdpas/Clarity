<script lang="ts">
  import { isFolded } from "./folding";
  import { goBack, goNext, shownStep, stepButtons, type StepButton, type StepPlace } from "./steps";
  import { untrack } from "svelte";
  import DateTimeInput from "./ui/DateTimeInput.svelte";
  import Table, { type CellKind, type TableColumn } from "./ui/Table.svelte";
  import CheckInput from "./ui/CheckInput.svelte";
  import ChoiceInput from "./ui/ChoiceInput.svelte";
  import DateInput from "./ui/DateInput.svelte";
  import NumberInput from "./ui/NumberInput.svelte";
  import SliderInput from "./ui/SliderInput.svelte";
  import ColorInput from "./ui/ColorInput.svelte";
  import FileInput, { type RecordFileValue } from "./ui/FileInput.svelte";
  import TagsInput from "./ui/TagsInput.svelte";
  import TypeAheadInput from "./ui/TypeAheadInput.svelte";
  import { searched, suggestions, type RefOption } from "./dropdownOptions";
  import type { Item, ItemKind } from "./listFields";
  import { numberRange } from "./ranges";
  import RadioInput from "./ui/RadioInput.svelte";
  import TextInput from "./ui/TextInput.svelte";
  import SegmentDisplay from "./ui/SegmentDisplay.svelte";
  import CanvasView from "./ui/CanvasView.svelte";
  import MarkdownView from "./ui/MarkdownView.svelte";
  import PartMenu from "./ui/PartMenu.svelte";
  import Icon from "./ui/Icon.svelte";
  import type { Command } from "./generated/Command";
  import { itemClicks, itemShown } from "./overlays";
  import { previewPopovers } from "./previewPopovers.svelte";
  import { linkAddress, progressFull } from "./smallParts";
  import { pressedSection, shownSection } from "./accordions";
  import { openLink } from "./backend";
  import { DEFAULT_DIGITS } from "./display";
  import { DEFAULT_UNITS } from "./drawing";
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import type { JsonValue } from "./generated/serde_json/JsonValue";
  import { displayValue, fieldFormat, findEntity, findEnum, findField, formatsOf, isNumeric, itemText, keyFile, markdownOf, offeredThemes, ownWord, shownText, themeOf, tr } from "./model";
  import { styleClass } from "./theme";
  import { languageName, languagesOf } from "./translate";
  import { numberEditing } from "./format";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { GAP, allParts, rowAt, boundField, fieldOf, findPart, helpId, partId, widgetInfo, withProps, type PageData } from "./pages";
  import type { Event as PartEvent } from "./generated/Event";
  import { useLive } from "./live";
  import { provideForm, useForm } from "./form";
  import { matchesShortcut } from "./shortcut";
  import { wiredCall } from "./calls";
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
  // A foldable panel or form: folded as the person or a handler left it, or as designed.
  let foldedHere = $state<boolean | null>(null);
  const folded = $derived(p.foldable && (live ? isFolded(p.folded, partState) : (foldedHere ?? p.folded)));

  function toggleFold() {
    const next = !folded;
    // Live, the part's state carries it to the handlers; in the preview it is the window's own.
    if (live && p.name) live.setFolded(p.name, next);
    else foldedHere = next;
  }
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

  /** Whether a key press lands in a field the person types in. */
  function typingIn(target: EventTarget | null): boolean {
    if (target instanceof HTMLTextAreaElement) return true;
    if (target instanceof HTMLElement && target.isContentEditable) return true;
    return target instanceof HTMLInputElement && !["checkbox", "radio", "button", "submit", "reset", "range", "color", "file"].includes(target.type);
  }

  // A button's shortcut, and Enter and Escape for the default and cancel
  // buttons, work anywhere in the running window.
  function windowKey(e: KeyboardEvent) {
    if (design || !live || p.kind !== "button" || hidden || disabled || e.defaultPrevented) return;
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
    const inTextArea = e.target instanceof HTMLTextAreaElement;
    const onButton = e.target instanceof HTMLButtonElement;
    const hit =
      (p.shortcut !== null && matchesShortcut(e, p.shortcut, typingIn(e.target))) ||
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
    if (!bound) return;
    bound.record[bound.field.name.name] = next;
    // A setting is kept at once, and switches the language or theme it names.
    if (live && !design && p.bind?.split(".")[0] === "settings") live.keepSettings();
  }

  // A picture field: its stored picture comes on its own; a new one is
  // chosen from a file and kept in the field until the record is saved.
  const boundEntity = $derived(p.bind ? data[p.bind.split(".")[0]]?.entity.name.name : undefined);
  const canLoadPicture = $derived(!!live && !design && !readonly && !disabled);
  let fileInput = $state<HTMLInputElement | null>(null);
  let pictureProblem = $state(false);

  /** The bytes of a record's file, as base64: new ones as chosen, a stored one fetched; then `use` them. */
  async function withFileData(file: RecordFileValue | null, use: (name: string, data: string) => Promise<void> | undefined) {
    if (!file || !live || design) return;
    const data = file.data ?? (file.stored && bound && boundEntity ? await live.fileData(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name) : null);
    if (data) await use(file.name, data);
  }

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

  // A slider's or a spin box's range: as written, else the bound field's own.
  const range = $derived(numberRange(p, bound?.field.modifiers));
  const whole = $derived(bound ? bound.field.type.kind === "integer" : !range.step.includes(".") && !(range.min ?? "").includes("."));

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

  // A wizard: the step its part's state is on, or in the designer the step picked.
  let stepHere = $state(1);
  const stepPlace: StepPlace = {
    get: () => (live && p.name ? live.parts[p.name]?.step : stepHere),
    set: (step) => {
      if (live && p.name) live.setStep(p.name, step);
      else stepHere = step;
    },
  };
  const shownStepIndex = $derived(design ? activeTab : shownStep(live && p.name ? partState?.step : stepHere, widget.pages.length) - 1);
  const buttonsOfStep = $derived(stepButtons(shownStepIndex + 1, widget.pages.length));

  function pressStep(button: StepButton) {
    if (design) return;
    const count = widget.pages.length;
    if (button === "back") goBack(stepPlace, count);
    else if (button === "next") void goNext(stepPlace, count, wiredCall(p.handlers, "next") !== null ? () => fire("next") : undefined);
    else void fire("finish");
  }

  function click() {
    if (design) return;
    if (p.handlers.some((h) => h.event === "click")) fire("click");
    else if (p.onClick && findPart(page, p.onClick)?.props.kind === "popover") openPopover(p.onClick);
    else if (p.onClick) onnavigate(p.onClick, p.newWindow);
  }

  /** A link: its address, the address it binds, or its click. */
  function followLink() {
    if (design) return;
    const address = p.to ?? (p.bind ? linkAddress(value) : null);
    if (address) openLink(address);
    else if (!p.to && !p.bind) click();
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

  // An accordion's open section where no application runs; live, the part's state keeps it.
  let openHere = $state(1);

  /** A press on the header of section `press` (from 1): it opens, or folds when it was open. */
  function pressSection(press: number) {
    if (design) return;
    const count = widget.pages.length;
    if (live && p.name) {
      live.setOpen(p.name, pressedSection(partState?.open, press, count));
      void fire("change");
    } else openHere = pressedSection(openHere, press, count);
  }

  /** A split button's arrow: its menu, below the button. */
  function openSplit(e: MouseEvent) {
    if (design || !widget.menu) return;
    const at = (e.currentTarget as HTMLElement).parentElement!.getBoundingClientRect();
    menuAt = { x: at.left, y: at.bottom + 2 };
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

  /** A column's width dragged in the designer: written into the part, 0 taking it away. */
  function designWidth(field: string, width: number) {
    const widths = { ...p.widths };
    if (width > 0) widths[field] = width;
    else delete widths[field];
    pageDesigner.edit({ op: "updateWidget", item: widget.span.start, container: null, props: withProps(p, { widths }) });
  }

  /** The table's columns for the library part: how each field shows and is edited. */
  const tableColumns = $derived.by((): TableColumn[] => {
    if (!["table", "tree", "treeTable"].includes(p.kind) || !collection) return [];
    const entity = collection.entity;
    // A tree shows the fields of `show`, else its record's text.
    const display = entity.fields.find((f) => f.type.kind === "text" || f.type.kind === "email")?.name.name ?? "id";
    const names = p.kind === "tree" ? (p.show.length ? p.show : [display]) : p.columns;
    return names.map((name) => {
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
        width: p.widths[name],
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
      case "color":
        return "color";
      default:
        return "text";
    }
  }

  /** An enum's values (never empty), or the records a reference can point at after an empty choice. */
  function choicesOf(field: NonNullable<ReturnType<typeof findField>>): TableColumn["choices"] {
    const type = field.type;
    // The settings' own: the project's languages by their own names, the offered themes by their titles.
    if (type.kind === "language") return languagesOf(preview.project.locale.language, Object.keys(preview.project.translations)).map((l) => ({ value: l, label: languageName(l) }));
    if (type.kind === "theme") return offeredThemes(preview).map((t) => ({ value: t.name.name, label: tr(preview, t.label) }));
    if (type.kind === "enum") return (findEnum(preview, type.name.name)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(preview, v.label) }));
    if (type.kind !== "ref") return [];
    // Running, a reference holds the id of its record; in the designer, the sample's text.
    const records = live
      ? live.refOptions(type.entity.name).map((o) => ({ value: o.id, label: o.text }))
      : (preview.displays[type.entity.name] ?? []).map((text) => ({ value: text, label: text }));
    return [{ value: null, label: "" }, ...records];
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

</script>

<svelte:window onkeydown={windowKey} />

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
  onpointerdown={pressMove}
  ondblclick={(e) => {
    if (!design || p.made) return;
    e.stopPropagation();
    pageDesigner.openPartCode(offset);
  }}
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
    {:else if p.kind === "separator"}
      <span class="separator-line" class:vertical={p.orientation === "vertical"} role="separator" aria-orientation={p.orientation === "vertical" ? "vertical" : "horizontal"}></span>
    {:else if p.kind === "badge"}
      <span class="badge {partClass ?? ''}" {id}>{bound ? displayValue(preview, bound.field, value) : (caption ?? "")}</span>
    {:else if p.kind === "link"}
      <button type="button" class="link" {id} {disabled} tabindex={p.tabOrder ?? undefined} onclick={followLink}
        >{bound ? displayValue(preview, bound.field, value) : (caption ?? "")}</button
      >
    {:else if p.kind === "progress"}
      {@const full = progressFull(value, p.max)}
      <span class="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(full * 100)} {id}>
        <span class="progress-fill" style:width="{full * 100}%"></span>
      </span>
    {:else if p.kind === "busy"}
      <span class="busy" role="status" {id}><span class="spinner"></span>{caption ?? ""}</span>
    {:else if p.kind === "icon"}
      <span class="icon-part" title={p.tooltip ? tr(preview, p.tooltip) : undefined}><Icon paths={preview.icons} name={p.icon ?? ""} size={20} /></span>
    {:else if p.kind === "heading"}
      <h2 class="heading-text">{#if unset && !caption}<span class="unset">{unset}</span>{/if}{caption ?? ""}</h2>
    {:else if p.kind === "text"}
      <span class="text-value" class:lines={p.height > 1} {id}>{bound ? displayValue(preview, bound.field, value) : ""}</span>
    {:else if p.kind === "markdown"}
      <!-- A file of the resources; a key's file in the language shown (a generated application has it as the key's text); or what it binds. -->
      {@const keyed = p.caption ? keyFile(preview, p.caption) : undefined}
      <MarkdownView
        {id}
        text={p.source ? markdownOf(preview, p.source) : keyed ? markdownOf(preview, keyed) : p.caption ? tr(preview, p.caption) : typeof value === "string" ? value : ""}
        onlink={openLink}
      />
    {:else if p.kind === "display"}
      <SegmentDisplay {id} value={bound ? value : null} digits={p.digits ?? DEFAULT_DIGITS} />
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
    {:else if p.kind === "spinbox"}
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
        {range}
        {whole}
        onchange={(next) => {
          if (bound) setValue(next);
          else setText(String(next));
          fire("change");
        }}
        onsubmit={submit}
        onfocus={() => fire("enter")}
        onblur={() => fire("leave")}
      />
    {:else if p.kind === "file"}
      {@const kept = (bound ? value : null) as RecordFileValue | null}
      <FileInput
        {id}
        value={kept}
        words={{
          none: ownWord(preview, "No file"),
          choose: ownWord(preview, "Choose a file"),
          open: ownWord(preview, "Open"),
          save: ownWord(preview, "Save a copy"),
          remove: ownWord(preview, "Remove the file"),
        }}
        {describedBy}
        disabled={design || disabled}
        {readonly}
        onchoose={(name, data) => {
          setValue({ name, data });
          fire("change");
        }}
        onremove={() => {
          setValue(null);
          fire("change");
        }}
        onopen={() => withFileData(kept, (name, data) => live?.openFile(name, data))}
        onsave={() => withFileData(kept, (name, data) => live?.saveFile(name, data))}
      />
    {:else if p.kind === "tags"}
      {@const type = bound?.field.type}
      {@const listed = type?.kind === "values" ? type.item : null}
      <TagsInput
        {id}
        items={(bound ? value : []) as Item[] | null}
        kind={(listed?.kind ?? "text") as ItemKind}
        choices={listed?.kind === "enum" ? (findEnum(preview, listed.name.name)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(preview, v.label) })) : []}
        show={(item) => (bound ? itemText(preview, bound.field, item) : String(item))}
        words={{ add: ownWord(preview, "Add"), remove: ownWord(preview, "Remove") }}
        {describedBy}
        disabled={design || disabled}
        {readonly}
        onchange={(next) => {
          setValue(next);
          fire("change");
        }}
      />
    {:else if p.kind === "color"}
      <ColorInput
        {id}
        value={bound ? (value as string | null) : shown}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onchange={(next) => {
          if (bound) setValue(next);
          else setText(next);
          fire("change");
        }}
      />
    {:else if p.kind === "slider"}
      <SliderInput
        {id}
        value={bound ? (value as string | number | null) : shown}
        {range}
        {whole}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        {disabled}
        {readonly}
        onchange={(next) => {
          if (bound) setValue(next);
          else setText(String(next));
          fire("change");
        }}
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
    {:else if p.kind === "dropdown" && p.typed}
      <!-- Any text, the values the field already has offered (U1c). -->
      {@const owner = p.bind ? data[p.bind.split(".")[0]]?.entity.name.name : undefined}
      {@const fieldName = bound?.field.name.name}
      {@const known = owner && fieldName ? (live ? live.fieldValues(owner, fieldName) : (preview.samples[owner] ?? []).map((r) => r[fieldName]).filter((v): v is string => typeof v === "string")) : []}
      <TypeAheadInput
        {id}
        text={typeof value === "string" ? value : ""}
        free
        offer={(typed) =>
          suggestions(known, typed).map((v) => ({ key: v, cells: [v] }))}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onpick={(next) => {
          setValue(next);
          fire("change");
        }}
        oncommit={(next) => {
          setValue(next);
          fire("change");
        }}
      />
    {:else if p.kind === "dropdown" && p.search && bound?.field.type.kind === "ref"}
      <!-- A record among many, filtered as it is typed, with the fields it shows (U1c). -->
      {@const target = bound.field.type.entity.name}
      {@const records: RefOption[] = live ? live.refOptions(target) : (preview.displays[target] ?? []).map((text, i) => ({ id: i + 1, text }))}
      {@const targetEntity = findEntity(preview, target)}
      {@const shownFields = p.show.map((f) => (targetEntity ? findField(targetEntity, f) : undefined))}
      {@const cellsOf = (o: RefOption) => p.show.map((f, i) => (shownFields[i] ? displayValue(preview, shownFields[i]!, (o.fields?.[f] ?? null) as JsonValue) : ""))}
      <TypeAheadInput
        {id}
        text={records.find((o) => o.id === value || o.text === value)?.text ?? ""}
        offer={(typed) => searched(records, typed, cellsOf).map((o) => ({ key: String(o.id), cells: [o.text, ...cellsOf(o)] }))}
        tabindex={p.tabOrder ?? undefined}
        {describedBy}
        invalid={error !== null}
        {disabled}
        {readonly}
        onpick={(key) => {
          const picked = records.find((o) => String(o.id) === key);
          setValue(live ? (picked?.id ?? null) : (picked?.text ?? null));
          fire("change");
        }}
        onclear={() => {
          setValue(null);
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
      {#if p.split}
        <!-- A split button: its click, and an arrow that opens its menu below it. -->
        <span class="split">
          <button {id} class="push {partClass ?? ''}" class:default={p.default} {disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={p.shortcut ?? undefined} onclick={click}
            >{#if p.icon}<Icon paths={preview.icons} name={p.icon} />{/if}{caption ?? ""}</button
          ><button type="button" class="push arrow {partClass ?? ''}" {disabled} aria-label="More" aria-haspopup="menu" onclick={openSplit}
            ><Icon paths={{ down: "M 6 9 l 6 6 l 6 -6" }} name="down" /></button
          >
        </span>
      {:else}
        <button {id} class="push {partClass ?? ''}" class:default={p.default} {disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={p.shortcut ?? undefined} onclick={click}
          >{#if p.icon}<Icon paths={preview.icons} name={p.icon} />{/if}{caption ?? ""}</button
        >
      {/if}
    {:else if p.kind === "toggle"}
      {@const pressed = bound ? value === true : partState?.checked === true}
      <button
        type="button"
        {id}
        class="push toggle {partClass ?? ''}"
        class:pressed
        aria-pressed={pressed}
        disabled={disabled || readonly}
        tabindex={p.tabOrder ?? undefined}
        title={!caption && p.name ? p.name : undefined}
        onclick={() => {
          if (design) return;
          if (bound) setValue(!pressed);
          else if (live && p.name) live.setChecked(p.name, !pressed);
          fire("change");
        }}>{#if p.icon}<Icon paths={preview.icons} name={p.icon} />{/if}{caption ?? ""}</button
      >
    {:else if p.kind === "accordion"}
      {@const open = design ? activeTab + 1 : shownSection(live && p.name ? partState?.open : openHere, widget.pages.length)}
      <div class="accordion">
        {#each widget.pages as section, index (section.span.start)}
          <button
            type="button"
            class="section-header"
            class:open={open === index + 1}
            class:selected={design && pageDesigner.selected === section.span.start}
            aria-expanded={open === index + 1}
            data-tab-switch="{offset}:{index}"
            {disabled}
            onpointerdown={(e) => design && pressTab(e, index, section.span.start)}
            onclick={() => pressSection(index + 1)}
            ><span class="chevron" class:closed={open !== index + 1}>&#9662;</span>{section.title ? tr(preview, section.title) : ""}</button
          >
          {#if open === index + 1}
            <div class="section-page">
              <GridBox {preview} {page} {data} {design} widgets={section.children} columns={p.width} container={section.span.start} {onnavigate} />
            </div>
          {/if}
        {/each}
      </div>
    {:else if p.kind === "scroll"}
      <div class="scroll-box">
        <GridBox {preview} {page} {data} {design} widgets={widget.children} columns={p.width} container={offset} {onnavigate} />
      </div>
    {:else if p.kind === "buttons"}
      <div class="buttons" role="group">
        <GridBox {preview} {page} {data} {design} widgets={widget.children} columns={p.width} fixedRows={1} joined container={offset} {onnavigate} />
      </div>
    {:else if p.kind === "panel" || p.kind === "form"}
      <fieldset class={p.kind} class:bare={p.kind === "form" && !p.caption && !p.foldable} class:folded={folded && !design}>
        {#if p.foldable}
          <!-- The person folds and unfolds it by its header; designing, it stays open to edit. -->
          <legend>
            <button type="button" class="fold" aria-expanded={!folded} onclick={toggleFold} disabled={design}>
              <span class="chevron" class:closed={folded}>&#9662;</span>{p.caption ? tr(preview, p.caption) : ""}
            </button>
          </legend>
        {:else if p.caption}<legend>{tr(preview, p.caption)}</legend>{/if}
        {#if !folded || design}
          <GridBox {preview} {page} {data} {design} widgets={widget.children} columns={p.width} container={offset} {onnavigate} />
        {/if}
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
    {:else if p.kind === "steps"}
      <div class="steps">
        <div class="step-titles">
          {#each widget.pages as step, index (step.span.start)}
            {#if design}
              <button
                class="step-title"
                class:current={index === shownStepIndex}
                class:selected={pageDesigner.selected === step.span.start}
                data-tab-switch="{offset}:{index}"
                onpointerdown={(e) => pressTab(e, index, step.span.start)}>{step.title ? tr(preview, step.title) : step.title}</button
              >
            {:else}
              <span class="step-title" class:current={index === shownStepIndex} class:done={index < shownStepIndex} aria-current={index === shownStepIndex ? "step" : undefined}
                >{step.title ? tr(preview, step.title) : step.title}</span
              >
            {/if}
          {/each}
        </div>
        {#if widget.pages[shownStepIndex]}
          {@const step = widget.pages[shownStepIndex]}
          <div class="step-page">
            <GridBox {preview} {page} {data} {design} widgets={step.children} columns={p.width} container={step.span.start} {onnavigate} />
          </div>
        {/if}
        <div class="step-buttons">
          {#each buttonsOfStep as button (button)}
            <button type="button" {disabled} onclick={() => pressStep(button)}
              >{ownWord(preview, button === "back" ? "Back" : button === "next" ? "Next" : "Finish")}</button
            >
          {/each}
        </div>
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
        resizable={p.resizable}
        sortable={p.sortable}
        order={p.order}
        remember={p.name ? `clarity-columns.${page.name.name}.${p.name}` : null}
        page={p.page}
        pageWords={{ first: ownWord(preview, "First page"), previous: ownWord(preview, "Previous page"), next: ownWord(preview, "Next page"), last: ownWord(preview, "Last page") }}
        ondesignwidth={design ? designWidth : undefined}
        onselect={(row) => selectRow(row)}
        onopen={() => fire("open")}
        onedit={() => fire("change")}
      />
    {:else if p.kind === "tree" || p.kind === "treeTable"}
      <!-- A tree (U1c): the rows under their parents, by the one reference to their own kind. -->
      {@const own = collection?.entity}
      {@const parent = own?.fields.find((f) => f.type.kind === "ref" && f.type.entity.name === own.name.name)?.name.name ?? "parent"}
      <Table
        columns={tableColumns}
        rows={collection?.rows ?? []}
        selected={selectedRow ?? null}
        {design}
        {disabled}
        {readonly}
        resizable={p.resizable}
        header={p.kind === "treeTable"}
        tree={{
          parent,
          open: ownWord(preview, "Expand"),
          close: ownWord(preview, "Collapse"),
          // In the designer a sample reference holds the text of the row it points at.
          key: live ? "id" : own?.fields.find((f) => f.type.kind === "text")?.name.name,
        }}
        remember={p.name ? `clarity-columns.${page.name.name}.${p.name}` : null}
        onselect={(row) => selectRow(row)}
        onopen={() => fire("open")}
        onedit={() => fire("change")}
      />
    {:else if p.kind === "list"}
      <ul class="list">
        {#each collection?.rows ?? [] as row, i (i)}
          <li class:chosen={selectedRow === i} role="presentation" onclick={() => selectRow(i)} oncontextmenu={() => selectRow(i)} ondblclick={() => fire("open")}>{p.show.map((f) => String(row[f] ?? "")).join(" | ")}</li>
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
            <span>{ownWord(preview, "No picture")}</span>
          </div>
        {/if}
        {#if canLoadPicture}
          <div class="picture-actions">
            <button type="button" onclick={() => fileInput?.click()}>{ownWord(preview, "Load a picture")}</button>
            {#if value}
              <button type="button" onclick={removePicture}>{ownWord(preview, "Remove the picture")}</button>
            {/if}
          </div>
          <input bind:this={fileInput} type="file" accept="image/png,image/jpeg,image/gif,image/bmp,image/webp" hidden onchange={pictureChosen} />
        {/if}
      </div>
    {:else if p.kind === "image"}
      {@const named = p.source ?? (p.caption ? (keyFile(preview, p.caption) ?? tr(preview, p.caption)) : null)}
      {@const picture = named ? pictureUrl(named) : null}
      {#if picture}
        <img class="picture" src={picture} alt={p.tooltip ? tr(preview, p.tooltip) : ""} />
      {:else}
        <div class="placeholder image">
          <span>Image</span>
          <code>{named ?? "no source"}</code>
        </div>
      {/if}
    {:else if p.kind === "canvas" && partState?.drawing}
      <CanvasView
        {id}
        drawing={partState.drawing}
        units={p.units ?? { width: DEFAULT_UNITS, height: DEFAULT_UNITS }}
        shown={(text) => shownText(preview, text)}
        onpoint={live && p.name && !design && p.handlers.some((h) => h.event === "click")
          ? (point) => {
              // Where it was clicked goes with the part's state, for the handler to read.
              if (live && p.name) live.setPointer(p.name, point);
              fire("click");
            }
          : null}
      />
    {:else if p.kind === "canvas"}
      <div class="placeholder canvas">
        <span>Canvas ({p.surface ?? "svg"})</span>
        <code>{p.renderer ?? "no renderer"}</code>
      </div>
    {/if}
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

  /* Over several rows: lines to read, wrapped from the top, with their
     paragraphs, scrolling when longer than the rows. */
  .text-value.lines {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    overflow: auto;
    text-overflow: clip;
    align-self: stretch;
    height: 100%;
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
    margin: 0;
    padding: 0;
  }

  .widget.design > .content > fieldset.form.bare {
    outline: 1px dashed var(--border-strong);
  }

  /* An icon stands beside the caption. */
  .push {
    width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
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
  .accordion > .content,
  .scroll > .content,
  .steps > .content,
  .splitter > .content,
  .table > .content,
  .list > .content,
  .image > .content,
  .canvas > .content {
    align-items: stretch;
  }

  /* A paged table's bar goes below it. */
  .table > .content {
    flex-direction: column;
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

  /* A framed container's border and padding sit out in the space between
     columns (GAP 8px: 3px each side, leaving 2px between frames side by
     side), so its own grid is exactly as wide as its span and its column
     lines are the window's. */
  fieldset {
    flex: 1;
    min-width: 0;
    margin: 0 -3px;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    padding: calc(var(--padding) * 2) calc(3px - var(--border-width));
  }

  legend {
    padding: 0 6px;
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  /* The header of a foldable panel or form, which folds and unfolds it. */
  legend .fold {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    color: inherit;
    cursor: pointer;
  }

  legend .fold:disabled {
    cursor: inherit;
    opacity: 1;
  }

  .chevron {
    display: inline-block;
    transition: transform 0.1s;
  }

  .chevron.closed {
    transform: rotate(-90deg);
  }

  fieldset.folded {
    flex: none;
    align-self: flex-start;
    padding-bottom: 0;
  }

  .tabs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    /* The tab page's frame sits out between the columns, as a panel's does. */
    margin: 0 -3px;
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

  .accordion {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 2px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
  }

  .section-header.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .section-page {
    flex: 1;
    min-height: 0;
    padding: calc(var(--padding) * 2) 0;
    overflow: hidden;
  }

  /* The scroll bar stands in the space between columns, so the parts keep the window's column lines. */
  .scroll-box {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    margin-right: -8px;
    padding-right: 8px;
    scrollbar-width: thin;
  }

  .split {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .split > .push:first-child {
    flex: 1;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }

  .split > .arrow {
    width: auto;
    flex: none;
    padding: 0 6px;
    border-left-width: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }

  .toggle.pressed {
    background: var(--accent-soft, var(--panel-alt));
    border-color: var(--accent);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.2);
  }

  .buttons {
    flex: 1;
    display: flex;
    min-width: 0;
  }

  /* Joined: only the outer corners of the group are round. */
  .buttons :global(.widget .push) {
    border-radius: 0;
  }

  .buttons :global(.widget:not(:first-child) .push) {
    margin-left: calc(-1 * var(--border-width, 1px));
  }

  .buttons :global(.widget:first-child .push) {
    border-top-left-radius: var(--radius-button);
    border-bottom-left-radius: var(--radius-button);
  }

  .buttons :global(.widget:last-child .push) {
    border-top-right-radius: var(--radius-button);
    border-bottom-right-radius: var(--radius-button);
  }

  .separator-line {
    flex: 1;
    align-self: center;
    height: var(--border-width, 1px);
    background: var(--border-strong);
  }

  .separator-line.vertical {
    flex: none;
    align-self: stretch;
    width: var(--border-width, 1px);
    height: auto;
    margin: 0 auto;
  }

  .badge {
    align-self: center;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--font-size-small);
    white-space: nowrap;
  }

  .link {
    align-self: center;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    text-align: left;
    font: inherit;
  }

  .link:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  .progress {
    flex: 1;
    align-self: center;
    height: 8px;
    border-radius: 4px;
    background: var(--panel-alt);
    border: var(--border-width, 1px) solid var(--border);
    overflow: hidden;
  }

  .progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  .busy {
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .icon-part {
    flex: 1;
    display: grid;
    place-items: center;
  }

  .steps {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    min-width: 0;
  }

  .step-titles {
    display: flex;
    gap: calc(var(--padding) * 3);
    border-bottom: var(--border-width) solid var(--border-strong);
    padding-bottom: var(--padding);
  }

  .step-title {
    color: var(--text-muted);
    background: none;
    border: none;
    padding: 0;
  }

  .step-title.done {
    color: var(--text);
  }

  .step-title.current {
    color: var(--accent);
    font-weight: 600;
  }

  .step-title.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .step-page {
    flex: 1;
    overflow: hidden;
  }

  .step-buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--padding);
  }

  .tab-page {
    flex: 1;
    border: var(--border-width) solid var(--border-strong);
    border-top: none;
    border-radius: 0 0 var(--radius-panel) var(--radius-panel);
    padding: calc(var(--padding) * 2) calc(3px - var(--border-width));
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
