<script lang="ts">
  import FrameProperties from "./FrameProperties.svelte";
  import TextField from "./TextField.svelte";
  import type { App } from "./generated/App";
  import type { Edit } from "./generated/Edit";
  import type { Event as PartEvent } from "./generated/Event";
  import { eventHandler, freePartName, wireEvent } from "./codeParts";
  import type { Orientation } from "./generated/Orientation";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Surface } from "./generated/Surface";
  import type { WidgetProps } from "./generated/WidgetProps";
  import { findFrameElement } from "./frame";
  import { WINDOW_HEIGHT, WINDOW_MAX_HEIGHT, WINDOW_MAX_WIDTH, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH } from "./grid";
  import { cellEdits, describeType, themeStyles } from "./model";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { allPages, allParts, locateInnerPage, locateWidget, widgetInfo, withProps } from "./pages";
  import { DEFAULT_DIGITS } from "./display";
  import { PLAIN_TYPES, dataEntity, dataType, isList, valueField } from "./windowData";
  import { resourceChoices } from "./resources";
  import { resourceKinds } from "./resourceKinds.svelte";

  let {
    preview,
    page,
    onedit,
    onapplied,
    app = null,
  }: {
    preview: Preview;
    page: Page;
    onedit: (edit: Edit) => void;
    /** Takes the project after a text's value or key changed in the language files. */
    onapplied: (preview: Preview) => void;
    /** Set when the page stands for the app (see `appAsPage`). */
    app?: App | null;
  } = $props();

  const selected = $derived(pageDesigner.selected);
  const location = $derived(selected === null ? null : locateWidget(preview, page, selected));
  /** The styles a part can take: those of the project's theme. */
  const styles = $derived(themeStyles(preview));
  const inner = $derived(selected === null || location ? null : locateInnerPage(page, selected));
  const p = $derived(location?.widget.props ?? null);
  const frameElement = $derived(selected === null || location || inner ? null : findFrameElement(page.frame, selected, page.widgets));
  const info = $derived(p ? widgetInfo(preview, p.kind) : null);

  const entities = $derived(preview.project.files.flatMap((f) => f.entities));
  const targets = $derived([
    ...allPages(preview).map((x) => x.name.name).filter((n) => n !== page.name.name),
  ]);

  /** Fields a widget can bind to, as `data.field`, from the page's single-record data. */
  const fieldOptions = $derived.by(() => {
    if (!info || info.bind !== "field") return [];
    return page.data
      .filter((d) => !isList(d))
      .flatMap((d) => {
        // A plain value binds by its name.
        const value = valueField(d);
        if (value) {
          const fits = info.fieldTypes.length === 0 || info.fieldTypes.includes(value.type.kind);
          return fits ? [{ value: d.name.name, label: `${d.name.name} (${dataType(d)})` }] : [];
        }
        const entity = entities.find((e) => e.name.name === dataEntity(d));
        return (entity?.fields ?? [])
          .filter((f) => info.fieldTypes.length === 0 || info.fieldTypes.includes(f.type.kind))
          .map((f) => ({ value: `${d.name.name}.${f.name.name}`, label: `${d.name.name}.${f.name.name} (${describeType(f)})` }));
      });
  });

  const listOptions = $derived(page.data.filter(isList).map((d) => d.name.name));
  const recordOptions = $derived(page.data.filter((d) => d.of.kind === "record").map((d) => d.name.name));

  /** Named inputs of the window, which a label or help text can belong to. */
  const inputNames = $derived(
    allParts(page)
      .filter((w) => w.props.name !== null && widgetInfo(preview, w.props.kind).input)
      .map((w) => w.props.name as string),
  );
  /** Named parts of the window a popover can open next to. */
  const partNames = $derived(
    allParts(page)
      .filter((w) => w.props.name !== null && w.props.kind !== "popover")
      .map((w) => w.props.name as string),
  );

  /** Fields of the entity behind a collection binding, for table columns and list items. */
  const collectionFields = $derived.by(() => {
    if (!p?.bind || p.bind.includes(".")) return [];
    const decl = page.data.find((d) => d.name.name === p.bind);
    const entity = entities.find((e) => e.name.name === (decl ? dataEntity(decl) : null));
    return (entity?.fields ?? []).filter((f) => f.type.kind !== "list");
  });

  /** Whose code behind the handlers go into. */
  const owner = $derived(app ? app.name.name : page.name.name);

  // The window's (or app's) own events, as the backend lists them.
  const ownerEvents = $derived(app ? preview.appEvents : preview.windowEvents);
  const ownerHandlers = $derived(app ? app.handlers : page.handlers);

  /** The handler name Wire suggests for each own event. */
  const OWNER_HANDLERS: Partial<Record<PartEvent, string>> = {
    open: "windowOpened",
    close: "windowClosed",
    return: "windowReturned",
    start: "appStarted",
  };

  /** Wires one of the window's or app's own events, or unwires it with `null`. */
  function setOwnerHandler(event: PartEvent, handler: string | null) {
    const target = app ? app.span.start : page.span.start;
    wire({ op: "setOwnerEvent", owner: target, event, handler }, handler);
  }

  function update(changes: Partial<WidgetProps>) {
    if (!location || !p) return;
    onedit({ op: "updateWidget", item: location.widget.span.start, container: null, props: withProps(p, changes) });
  }

  /** Sends an edit that wires `handler`, together with an empty handler in the code behind, and shows it. */
  function wire(edit: Edit, handler: string | null) {
    if (handler === null) {
      onedit(edit);
      return;
    }
    onedit({ op: "batch", edits: [edit, { op: "addHandler", owner, name: handler }] });
    pageDesigner.showCode(handler);
  }

  function toggle(list: string[], name: string, on: boolean): string[] {
    const next = on ? [...list, name] : list.filter((n) => n !== name);
    // Keep the entity's field order.
    return collectionFields.map((f) => f.name.name).filter((n) => next.includes(n));
  }

  /** Sets the window's size; an emptied field goes back to the standard size. */
  function setSize(width: string | number, height: string | number) {
    const w = Math.floor(Number(width));
    const h = Math.floor(Number(height));
    const standard = String(width).trim() === "" || String(height).trim() === "";
    onedit({ op: "setSize", page: page.span.start, width: standard ? null : w, height: standard ? null : h });
  }

  function numberOf(e: Event): number {
    return Math.max(1, Math.floor(Number((e.currentTarget as HTMLInputElement).value) || 1));
  }

  /** Wires a handler to an event, or unwires it with `null`. Handlers stay in catalog order. */
  /** The part's name, or the one it gets when it is changed: every part has one. */
  const partName = $derived(
    p?.name ?? (info ? freePartName(allParts(page).flatMap((w) => (w.props.name ? [w.props.name] : [])), info.keyword) : ""),
  );

  function setHandler(event: PartEvent, handler: string | null) {
    if (!p || !info || !location) return;
    onedit(wireEvent({ item: location.widget.span.start, props: { ...p, name: partName }, events: info.events, owner, event, handler }));
    if (handler) pageDesigner.showCode(handler);
  }

  function textOf(e: Event): string | null {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    return value === "" ? null : value;
  }

  // New data declaration form.
  let dataName = $state("");
  /** What new data holds: an entity's name, an enum's, or a plain type. */
  let dataOf = $state("");
  let dataList = $state(false);
  const enums = $derived(preview.project.files.flatMap((f) => f.enums).map((e) => e.name.name));
  /** Only records come as a list. */
  const listable = $derived(entities.some((e) => e.name.name === dataOf));

  function addData() {
    const name = dataName.trim();
    if (!name || !dataOf) return;
    onedit({ op: "addData", page: page.span.start, name, entity: dataOf, list: dataList && listable });
    dataName = "";
  }

  function withoutKey(widths: Record<string, number>, field: string): Record<string, number> {
    const next = { ...widths };
    delete next[field];
    return next;
  }

  /** A column's width as typed: a number from 20 to 2000, or empty for none. */
  function withWidth(widths: Record<string, number>, field: string, typed: string): Record<string, number> {
    const width = Math.round(Number(typed));
    if (typed.trim() === "" || !Number.isFinite(width)) return withoutKey(widths, field);
    return { ...widths, [field]: Math.min(2000, Math.max(20, width)) };
  }
</script>

<section>
  <!-- What is selected: a part, a tab or pane, a menu item, or else the window itself. -->
  {#if location && p && info}
    <div class="kind">{info.name}</div>

    <label class="prop">
      <span>Name</span>
      <input value={p.name ?? partName} title="The name the code behind uses" onchange={(e) => update({ name: textOf(e) ?? partName })} />
    </label>

    {#if p.kind === "markdown" || p.kind === "image"}
      <!-- A key whose value in each language is a file of the resources, set under i18n. -->
      <label class="prop">
        <span>Key</span>
        <input
          value={p.caption?.startsWith("@") ? p.caption.slice(1) : ""}
          placeholder={p.kind === "markdown" ? "gameRules" : "bannerPicture"}
          title="A key whose value in each language is a file of the resources; point it per language under i18n"
          spellcheck="false"
          onchange={(e) => {
            const key = textOf(e);
            update({ caption: key ? `@${key.replace(/^@/, "")}` : null });
          }}
        />
      </label>
    {:else if info.caption !== null || p.caption !== null || info.container === "plain" || info.properties.includes("for")}
      <label class="prop">
        <span>Caption</span>
        <TextField {preview} {onapplied} text={p.caption} placeholder={p.kind === "help" ? "the field's help text" : ""} onwrite={(text) => update({ caption: text })} />
      </label>
    {/if}

    <div class="position">
      <label><span>Column</span><input type="number" min="1" value={p.col} onchange={(e) => update({ col: numberOf(e) })} /></label>
      <label><span>Row</span><input type="number" min="1" value={p.row} onchange={(e) => update({ row: numberOf(e) })} /></label>
      <label><span>Width</span><input type="number" min="1" value={p.width} onchange={(e) => update({ width: numberOf(e) })} /></label>
      <label><span>Height</span><input type="number" min="1" value={p.height} onchange={(e) => update({ height: numberOf(e) })} /></label>
    </div>

    {#if info.bind === "field"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e) })}>
          <option value="">(not bound)</option>
          {#each fieldOptions as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      {#if fieldOptions.length === 0}
        <p class="hint">Declare data on the page first (select the page background).</p>
      {/if}
    {:else if info.bind === "record"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e) })}>
          <option value="">(not bound)</option>
          {#each recordOptions as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {:else if info.bind === "collection"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e), columns: [], edit: [], switches: [], show: [], widths: {}, order: null })}>
          <option value="">(not bound)</option>
          {#each listOptions as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if info.properties.includes("columns") || info.properties.includes("show")}
      {@const key = info.properties.includes("columns") ? "columns" : "show"}
      <fieldset class="checks">
        <legend>{key === "columns" ? "Columns" : "Shown fields"}</legend>
        {#each collectionFields as field (field.name.name)}
          <label>
            <input
              type="checkbox"
              checked={p[key].includes(field.name.name)}
              onchange={(e) =>
                update(
                  key === "columns" && !e.currentTarget.checked
                    ? {
                        columns: toggle(p.columns, field.name.name, false),
                        edit: toggle(p.edit, field.name.name, false),
                        switches: toggle(p.switches, field.name.name, false),
                        widths: withoutKey(p.widths, field.name.name),
                        order: p.order?.field === field.name.name ? null : p.order,
                      }
                    : { [key]: toggle(p[key], field.name.name, e.currentTarget.checked) },
                )}
            />
            {field.label}
            {#if key === "columns" && p.columns.includes(field.name.name)}
              <input
                class="width"
                type="number"
                min="20"
                max="2000"
                placeholder="auto"
                aria-label="Width of {field.label} in pixels"
                title="Width in pixels; empty shares what is left. The header edges on the canvas set it too."
                value={p.widths[field.name.name] ?? ""}
                onchange={(e) => update({ widths: withWidth(p.widths, field.name.name, e.currentTarget.value) })}
              />
            {/if}
          </label>
        {:else}
          <p class="hint">Bind to a list first.</p>
        {/each}
      </fieldset>
    {/if}

    {#if info.properties.includes("edit") && p.columns.length > 0}
      {@const editable = collectionFields.filter((f) => p.columns.includes(f.name.name) && cellEdits(f))}
      <fieldset class="checks" title="These cells are inputs. Wire 'on change' to save the row, as in: save grid.selected">
        <legend>Edited in the table</legend>
        {#each editable as field (field.name.name)}
          <label>
            <input
              type="checkbox"
              checked={p.edit.includes(field.name.name)}
              onchange={(e) =>
                update(
                  e.currentTarget.checked
                    ? { edit: toggle(p.edit, field.name.name, true) }
                    : { edit: toggle(p.edit, field.name.name, false), switches: toggle(p.switches, field.name.name, false) },
                )}
            />
            {field.label}
            {#if field.type.kind === "bool" && p.edit.includes(field.name.name)}
              <select
                class="cell-look"
                aria-label="How {field.label} shows"
                value={p.switches.includes(field.name.name) ? "switch" : "checkbox"}
                onchange={(e) => update({ switches: toggle(p.switches, field.name.name, e.currentTarget.value === "switch") })}
              >
                <option value="checkbox">Checkbox</option>
                <option value="switch">Switch</option>
              </select>
            {/if}
          </label>
        {:else}
          <p class="hint">None of the columns can be edited in a cell.</p>
        {/each}
      </fieldset>
    {/if}

    {#if info.properties.includes("sortable") && p.columns.length > 0}
      <fieldset class="checks">
        <legend>Table</legend>
        <label title="The person drags the header edges; the widths are remembered for them">
          <input type="checkbox" checked={p.resizable} onchange={(e) => update({ resizable: e.currentTarget.checked })} /> Resizable columns
        </label>
        <label title="A click on a header sorts by that column; another click the other way">
          <input type="checkbox" checked={p.sortable} onchange={(e) => update({ sortable: e.currentTarget.checked })} /> Sortable by a header click
        </label>
        <label class="order">
          <span>First order</span>
          <select value={p.order?.field ?? ""} onchange={(e) => update({ order: e.currentTarget.value ? { field: e.currentTarget.value, desc: p.order?.desc ?? false } : null })}>
            <option value="">as the data has them</option>
            {#each p.columns as column (column)}<option value={column}>{column}</option>{/each}
          </select>
        </label>
        {#if p.order}
          {@const order = p.order}
          <label><input type="checkbox" checked={order.desc} onchange={(e) => update({ order: { ...order, desc: e.currentTarget.checked } })} /> Descending</label>
        {/if}
      </fieldset>
    {/if}

    {#if info.clickable}
      <label class="prop">
        <span>On click, open</span>
        <select value={p.onClick ?? ""} onchange={(e) => update({ onClick: textOf(e), newWindow: textOf(e) ? p.newWindow : false })}>
          <option value="">(nothing)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
      {#if p.onClick}
        <label class="inline" title="Opens in a desktop window of its own. Without it, the window opens in place, on top of this one.">
          <input type="checkbox" checked={p.newWindow} onchange={(e) => update({ newWindow: e.currentTarget.checked })} /> In a new window
        </label>
      {/if}
    {/if}

    {#if info.properties.includes("for")}
      <label class="prop">
        <span>{p.kind === "popover" ? "Next to" : "For input"}</span>
        <select value={p.for ?? ""} onchange={(e) => update({ for: textOf(e) })}>
          <option value="">(none)</option>
          {#each p.kind === "popover" ? partNames : inputNames as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("digits")}
      <label class="prop">
        <span>Digits</span>
        <input
          type="number"
          min="1"
          max="24"
          value={p.digits ?? ""}
          placeholder={String(DEFAULT_DIGITS)}
          onchange={(e) => update({ digits: textOf(e) === null ? null : numberOf(e) })}
        />
      </label>
    {/if}
    {#if info.properties.includes("placeholder")}
      <label class="prop">
        <span>Placeholder</span>
        <TextField {preview} {onapplied} text={p.placeholder} placeholder="shown while the input is empty" onwrite={(text) => update({ placeholder: text })} />
      </label>
    {/if}
    <label class="prop">
      <span>Tooltip</span>
      <TextField {preview} {onapplied} text={p.tooltip} placeholder="shown when the pointer rests on it" onwrite={(text) => update({ tooltip: text })} />
    </label>
    <label class="prop">
      <span>Style</span>
      {#if styles.length > 0 || p.style}
        <select value={p.style ?? ""} onchange={(e) => update({ style: e.currentTarget.value || null })}>
          <option value="">none</option>
          {#each styles as s (s)}<option value={s}>{s}</option>{/each}
          {#if p.style && !styles.includes(p.style)}<option value={p.style}>{p.style} (not in the theme)</option>{/if}
        </select>
      {:else}
        <span class="muted">Name a theme with styles in the project file to style parts.</span>
      {/if}
    </label>
    {#if info.properties.includes("tabOrder")}
      <label class="prop">
        <span>Tab order</span>
        <input
          type="number"
          min="1"
          value={p.tabOrder ?? ""}
          placeholder="reading order"
          onchange={(e) => update({ tabOrder: textOf(e) === null ? null : numberOf(e) })}
        />
      </label>
    {/if}
    {#if info.properties.includes("shortcut")}
      <label class="prop">
        <span>Shortcut</span>
        <input value={p.shortcut ?? ""} placeholder="Ctrl+S" onchange={(e) => update({ shortcut: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("foldable")}
      <fieldset class="checks">
        <legend>Folding</legend>
        <label
          ><input type="checkbox" checked={p.foldable} onchange={(e) => update({ foldable: e.currentTarget.checked, folded: e.currentTarget.checked && p.folded })} /> The person
          can fold it by its header</label
        >
        <label><input type="checkbox" checked={p.folded} disabled={!p.foldable} onchange={(e) => update({ folded: e.currentTarget.checked })} /> Starts folded</label>
        <p class="hint">Code reads and sets it as <code>{partName}.folded</code>.</p>
      </fieldset>
    {/if}

    {#if info.properties.includes("readonly") || info.properties.includes("default") || info.properties.includes("cancel")}
      <fieldset class="checks">
        <legend>Behavior</legend>
        {#if info.properties.includes("readonly")}
          <label><input type="checkbox" checked={p.readonly} onchange={(e) => update({ readonly: e.currentTarget.checked })} /> Read-only</label>
        {/if}
        {#if info.properties.includes("default")}
          <label><input type="checkbox" checked={p.default} onchange={(e) => update({ default: e.currentTarget.checked })} /> Enter clicks it (default)</label>
        {/if}
        {#if info.properties.includes("cancel")}
          <label><input type="checkbox" checked={p.cancel} onchange={(e) => update({ cancel: e.currentTarget.checked })} /> Escape clicks it (cancel)</label>
        {/if}
      </fieldset>
    {/if}

    {#if info.events.length > 0}
      <fieldset class="checks">
        <legend>Events</legend>
        {#each info.events as ev (ev)}
          {@const handler = p.handlers.find((h) => h.event === ev)?.handler ?? ""}
          <div class="event-row">
            <span class="event-name">{ev}</span>
            <input value={handler} placeholder="handler" onchange={(e) => setHandler(ev, textOf(e))} />
            {#if handler}
              <span class="event-buttons">
                <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
                <button class="small" title="Unwire" onclick={() => setHandler(ev, null)}>&times;</button>
              </span>
            {:else}
              <button class="small" title="Wire a handler named after the part and the event" onclick={() => setHandler(ev, eventHandler(partName, ev))}>Wire</button>
            {/if}
          </div>
        {/each}
        <p class="hint">Handlers are written in Clarity, in the window's code behind.</p>
      </fieldset>
    {/if}

    {#if info.properties.includes("orientation")}
      <label class="prop">
        <span>Orientation</span>
        <select value={p.orientation ?? "horizontal"} onchange={(e) => update({ orientation: e.currentTarget.value as Orientation })}>
          <option value="horizontal">Side by side</option>
          <option value="vertical">Stacked</option>
        </select>
      </label>
    {/if}
    {#if info.properties.includes("surface")}
      <label class="prop">
        <span>Surface</span>
        <select value={p.surface ?? "svg"} onchange={(e) => update({ surface: e.currentTarget.value as Surface })}>
          <option value="svg">SVG (vector)</option>
          <option value="bitmap">Bitmap</option>
          <option value="gpu">GPU</option>
        </select>
      </label>
    {/if}
    {#if info.properties.includes("renderer")}
      <label class="prop">
        <span>Renderer</span>
        <input value={p.renderer ?? ""} placeholder="name of the hand-written renderer" onchange={(e) => update({ renderer: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("source")}
      <!-- A file of the resources, of the kind the part shows; a key points each language at one instead. -->
      {@const choices = resourceChoices(preview.project.resources?.files, p.kind === "markdown" ? "markdown" : "picture", resourceKinds.list)}
      <label class="prop">
        <span>Source</span>
        <select value={p.source ?? ""} onchange={(e) => update({ source: textOf(e) })}>
          <option value="">(none)</option>
          {#each choices as name (name)}<option value={name}>{name}</option>{/each}
          {#if p.source && !choices.includes(p.source)}<option value={p.source}>{p.source} (not in the resources)</option>{/if}
        </select>
      </label>
    {/if}

    {#if info.container === "tabs"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Tab ${location.widget.pages.length + 1}` })}>
        Add tab
      </button>
    {:else if info.container === "steps"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Step ${location.widget.pages.length + 1}` })}>
        Add step
      </button>
    {/if}

    <div class="actions">
      <button class="danger" onclick={() => onedit({ op: "remove", item: location.widget.span.start })}>Remove</button>
    </div>
  {:else if inner}
    {@const tab = inner.widget.pages[inner.index]}
    {#if tab.title !== null}
      {@const step = inner.widget.props.kind === "steps"}
      <div class="kind">{step ? "Step" : "Tab"}</div>
      <label class="prop">
        <span>Title</span>
        <TextField {preview} {onapplied} text={tab.title} onwrite={(text) => onedit({ op: "setTitle", target: tab.span.start, title: text ?? "" })} />
      </label>
      <div class="actions">
        <button class="danger" onclick={() => onedit({ op: "remove", item: tab.span.start })}>{step ? "Remove step" : "Remove tab"}</button>
      </div>
    {:else}
      <div class="kind">Pane</div>
    {/if}
  {:else if frameElement}
    <FrameProperties {preview} element={frameElement} {onedit} {onapplied} {owner} />
  {:else}
    {#if app}
      <div class="kind">App <code>{app.name.name}</code></div>
    {:else}
      <div class="kind">Page <code>{page.name.name}</code> <span class="muted">{page.kind}</span></div>
    {/if}
    <label class="prop">
      <span>Title</span>
      <TextField {preview} {onapplied} text={page.title} placeholder={page.name.name} onwrite={(text) => onedit({ op: "setTitle", target: page.span.start, title: text ?? "" })} />
    </label>
    {#if app}
      <label class="prop">
        <span>Main window</span>
        <select value={app.main?.name ?? ""} onchange={(e) => onedit({ op: "setMain", app: app.span.start, main: e.currentTarget.value })}>
          <option value="">(none)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
    {:else}
    <label class="prop">
      <span>Grid columns</span>
      <input type="number" min="1" max="24" value={page.columns} onchange={(e) => onedit({ op: "setGrid", page: page.span.start, columns: numberOf(e) })} />
    </label>
    <!-- The size its desktop window opens at; both empty for the standard size. -->
    <div class="prop size">
      <span>Size</span>
      <input type="number" min={WINDOW_MIN_WIDTH} max={WINDOW_MAX_WIDTH} aria-label="Width in pixels" placeholder={String(WINDOW_WIDTH)} value={page.size?.width ?? ""} onchange={(e) => setSize(e.currentTarget.value, page.size?.height ?? WINDOW_HEIGHT)} />
      <span class="times">x</span>
      <input type="number" min={WINDOW_MIN_HEIGHT} max={WINDOW_MAX_HEIGHT} aria-label="Height in pixels" placeholder={String(WINDOW_HEIGHT)} value={page.size?.height ?? ""} onchange={(e) => setSize(page.size?.width ?? WINDOW_WIDTH, e.currentTarget.value)} />
    </div>
    {/if}

    <fieldset class="checks">
      <legend>Data</legend>
      {#each page.data as decl (decl.span.start)}
        <div class="data-row">
          <code>{decl.name.name}: {dataType(decl)}</code>
          <button class="small" onclick={() => onedit({ op: "remove", item: decl.span.start })}>Remove</button>
        </div>
      {:else}
          <p class="hint">No data yet. Parts bind to the records, lists and values declared here.</p>
      {/each}
      <div class="add-data">
        <input bind:value={dataName} placeholder="name" />
        <select bind:value={dataOf} aria-label="What it holds">
          <option value="" disabled>holds</option>
          <optgroup label="Records">
            {#each entities as entity (entity.name.name)}
              <option value={entity.name.name}>{entity.name.name}</option>
            {/each}
          </optgroup>
          <optgroup label="Values">
            {#each PLAIN_TYPES as type (type)}
              <option value={type}>{type}</option>
            {/each}
            {#each enums as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </optgroup>
        </select>
        <label class="inline"><input type="checkbox" bind:checked={dataList} disabled={!listable} /> list</label>
        <button class="small" onclick={addData} disabled={!dataName.trim() || !dataOf}>Add</button>
      </div>
    </fieldset>

    <fieldset class="checks">
      <legend>Events</legend>
      {#each ownerEvents as ev (ev)}
        {@const handler = ownerHandlers.find((h) => h.event === ev)?.handler ?? ""}
        <div class="event-row">
          <span class="event-name">{ev}</span>
          <input value={handler} placeholder="handler" onchange={(e) => setOwnerHandler(ev, textOf(e))} />
          {#if handler}
            <span class="event-buttons">
              <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
              <button class="small" title="Unwire" onclick={() => setOwnerHandler(ev, null)}>&times;</button>
            </span>
          {:else}
            <button class="small" title="Wire a handler for this event" onclick={() => setOwnerHandler(ev, OWNER_HANDLERS[ev] ?? null)}
              >Wire</button
            >
          {/if}
        </div>
      {/each}
      <p class="hint">
        {app ? "Start runs once, before the main window opens." : "Open runs when the window has its data, close before it closes, return when a window it opened has closed: the place to reload a list."}
      </p>
    </fieldset>
  {/if}
</section>

<style>
  .checks .width {
    width: 6em;
    margin-left: auto;
  }

  .checks .order {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .kind {
    font-weight: 600;
  }

  .kind code {
    font-family: var(--mono);
    font-weight: 400;
    font-size: 12px;
  }

  .muted {
    color: var(--text-muted);
    font-weight: 400;
    font-size: 12px;
  }

  .prop {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Width x height on one line under the label. */
  .prop.size {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    column-gap: 6px;
  }

  .prop.size > span:first-child {
    grid-column: 1 / -1;
  }

  .prop.size input {
    min-width: 0;
  }

  .position {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }

  .position label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .checks {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin: 0;
    padding: 6px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .checks legend {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 4px;
  }

  .checks label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .event-row {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .event-buttons {
    display: flex;
    gap: 4px;
  }

  .event-name {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-muted);
  }

  .data-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }

  .data-row code {
    font-family: var(--mono);
    font-size: 12px;
  }

  .add-data {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 6px;
    align-items: center;
    margin-top: 4px;
  }

  .inline {
    font-size: 12px;
  }

  .small {
    padding: 2px 8px;
    font-size: 12px;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .danger {
    color: var(--error);
  }
  .cell-look {
    margin-left: auto;
    font-size: 11px;
    padding: 0 2px;
  }
</style>
