<script lang="ts">
  import FrameProperties from "./FrameProperties.svelte";
  import type { App } from "./generated/App";
  import type { Edit } from "./generated/Edit";
  import type { Event as PartEvent } from "./generated/Event";
  import type { Orientation } from "./generated/Orientation";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Surface } from "./generated/Surface";
  import type { WidgetProps } from "./generated/WidgetProps";
  import { findFrameElement } from "./frame";
  import { describeType, themeStyles } from "./model";
  import { pageDesigner } from "./pageDesigner.svelte";
  import { allPages, allParts, locateInnerPage, locateWidget, widgetInfo, withProps } from "./pages";

  let {
    preview,
    page,
    onedit,
    subject,
    app = null,
  }: {
    preview: Preview;
    page: Page;
    onedit: (edit: Edit) => void;
    /** The window itself, or what is picked on it. */
    subject: "window" | "picked";
    /** Set when the page stands for the app (see `appAsPage`). */
    app?: App | null;
  } = $props();

  const selected = $derived(pageDesigner.selected);
  const location = $derived(selected === null ? null : locateWidget(preview, page, selected));
  /** The styles a part can take: those of the project's theme. */
  const styles = $derived(themeStyles(preview));
  const inner = $derived(selected === null || location ? null : locateInnerPage(page, selected));
  const p = $derived(location?.widget.props ?? null);
  const frameElement = $derived(selected === null || location || inner ? null : findFrameElement(page.frame, selected));
  const info = $derived(p ? widgetInfo(preview, p.kind) : null);

  const entities = $derived(preview.project.files.flatMap((f) => f.entities));
  const targets = $derived([
    ...allPages(preview).map((x) => x.name.name).filter((n) => n !== page.name.name),
  ]);

  /** Fields a widget can bind to, as `data.field`, from the page's single-record data. */
  const fieldOptions = $derived.by(() => {
    if (!info || info.bind !== "field") return [];
    return page.data
      .filter((d) => !d.list)
      .flatMap((d) => {
        const entity = entities.find((e) => e.name.name === d.entity.name);
        return (entity?.fields ?? [])
          .filter((f) => info.fieldTypes.length === 0 || info.fieldTypes.includes(f.type.kind))
          .map((f) => ({ value: `${d.name.name}.${f.name.name}`, label: `${d.name.name}.${f.name.name} (${describeType(f)})` }));
      });
  });

  const listOptions = $derived(page.data.filter((d) => d.list).map((d) => d.name.name));
  const recordOptions = $derived(page.data.filter((d) => !d.list).map((d) => d.name.name));

  /** Named inputs of the window, which a label or help text can belong to. */
  const inputNames = $derived(
    allParts(page)
      .filter((w) => w.props.name !== null && widgetInfo(preview, w.props.kind).input)
      .map((w) => w.props.name as string),
  );

  /** Fields of the entity behind a collection binding, for table columns and list items. */
  const collectionFields = $derived.by(() => {
    if (!p?.bind || p.bind.includes(".")) return [];
    const decl = page.data.find((d) => d.name.name === p.bind);
    const entity = entities.find((e) => e.name.name === decl?.entity.name);
    return (entity?.fields ?? []).filter((f) => f.type.kind !== "list");
  });

  /** Whose code behind the handlers go into. */
  const owner = $derived(app ? app.name.name : page.name.name);

  // The window's (or app's) own events, as the backend lists them.
  const ownerEvents = $derived(app ? preview.appEvents : preview.windowEvents);
  const ownerHandlers = $derived(app ? app.handlers : page.handlers);

  /** The handler name Wire suggests for each own event. */
  const OWNER_HANDLERS: Partial<Record<PartEvent, string>> = {
    open: "window_opened",
    close: "window_closed",
    return: "window_returned",
    start: "app_started",
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

  function numberOf(e: Event): number {
    return Math.max(1, Math.floor(Number((e.currentTarget as HTMLInputElement).value) || 1));
  }

  /** Wires a handler to an event, or unwires it with `null`. Handlers stay in catalog order. */
  function setHandler(event: PartEvent, handler: string | null) {
    if (!p || !info) return;
    const others = p.handlers.filter((h) => h.event !== event);
    const handlers = handler ? [...others, { event, handler }] : others;
    handlers.sort((a, b) => info.events.indexOf(a.event) - info.events.indexOf(b.event));
    // A click runs a handler or opens a window, not both.
    const onClick = event === "click" && handler ? null : p.onClick;
    if (!location) return;
    const props = withProps(p, { handlers, onClick });
    wire({ op: "updateWidget", item: location.widget.span.start, container: null, props }, handler);
  }

  function textOf(e: Event): string | null {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    return value === "" ? null : value;
  }

  // New data declaration form.
  let dataName = $state("");
  let dataEntity = $state("");
  let dataList = $state(false);

  function addData() {
    const name = dataName.trim();
    if (!name || !dataEntity) return;
    onedit({ op: "addData", page: page.span.start, name, entity: dataEntity, list: dataList });
    dataName = "";
  }
</script>

<section>
  <h3>{subject === "window" ? "The window" : "What is picked"}</h3>

  {#if subject === "picked" && location && p && info}
    <div class="kind">{info.name}</div>

    <label class="prop">
      <span>Name</span>
      <input value={p.name ?? ""} placeholder="the name the code behind uses" onchange={(e) => update({ name: textOf(e) })} />
    </label>

    {#if info.caption !== null || p.caption !== null || info.container === "plain" || info.properties.includes("for")}
      <label class="prop">
        <span>Caption</span>
        <input
          value={p.caption ?? ""}
          placeholder={p.kind === "help" ? "the field's help text" : ""}
          onchange={(e) => update({ caption: textOf(e) })}
        />
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
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e), columns: [], show: [] })}>
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
              onchange={(e) => update({ [key]: toggle(p[key], field.name.name, e.currentTarget.checked) })}
            />
            {field.label}
          </label>
        {:else}
          <p class="hint">Bind to a list first.</p>
        {/each}
      </fieldset>
    {/if}

    {#if info.clickable}
      <label class="prop">
        <span>On click, open</span>
        <select value={p.onClick ?? ""} onchange={(e) => update({ onClick: textOf(e) })}>
          <option value="">(nothing)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if info.properties.includes("for")}
      <label class="prop">
        <span>For input</span>
        <select value={p.for ?? ""} onchange={(e) => update({ for: textOf(e) })}>
          <option value="">(none)</option>
          {#each inputNames as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("placeholder")}
      <label class="prop">
        <span>Placeholder</span>
        <input value={p.placeholder ?? ""} placeholder="shown while the input is empty" onchange={(e) => update({ placeholder: textOf(e) })} />
      </label>
    {/if}
    <label class="prop">
      <span>Tooltip</span>
      <input value={p.tooltip ?? ""} placeholder="shown when the pointer rests on it" onchange={(e) => update({ tooltip: textOf(e) })} />
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
              <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
              <button class="small" title="Unwire" onclick={() => setHandler(ev, null)}>&times;</button>
            {:else}
              <button
                class="small"
                disabled={!p.name}
                title={p.name ? "Wire a handler named after the part and the event" : "Give the part a name first"}
                onclick={() => setHandler(ev, p.name + "_" + ev)}>Wire</button
              >
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
      <label class="prop">
        <span>Source</span>
        <input value={p.source ?? ""} placeholder="logo.png" onchange={(e) => update({ source: textOf(e) })} />
      </label>
    {/if}

    {#if info.container === "tabs"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Tab ${location.widget.pages.length + 1}` })}>
        Add tab
      </button>
    {/if}

    <div class="actions">
      <button class="danger" onclick={() => onedit({ op: "remove", item: location.widget.span.start })}>Remove</button>
    </div>
  {:else if subject === "picked" && inner}
    {@const tab = inner.widget.pages[inner.index]}
    {#if tab.title !== null}
      <div class="kind">Tab</div>
      <label class="prop">
        <span>Title</span>
        <input value={tab.title} onchange={(e) => onedit({ op: "setTitle", target: tab.span.start, title: e.currentTarget.value })} />
      </label>
      <div class="actions">
        <button class="danger" onclick={() => onedit({ op: "remove", item: tab.span.start })}>Remove tab</button>
      </div>
    {:else}
      <div class="kind">Pane</div>
    {/if}
  {:else if subject === "picked" && frameElement}
    <FrameProperties {preview} element={frameElement} {onedit} {owner} />
  {:else if subject === "picked"}
    <p class="hint">Nothing yet. Click a part of the window, or a line in the Window tab.</p>
  {:else}
    {#if app}
      <div class="kind">App <code>{app.name.name}</code></div>
    {:else}
      <div class="kind">Page <code>{page.name.name}</code> <span class="muted">{page.kind}</span></div>
    {/if}
    <label class="prop">
      <span>Title</span>
      <input value={page.title ?? ""} placeholder={page.name.name} onchange={(e) => onedit({ op: "setTitle", target: page.span.start, title: e.currentTarget.value })} />
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
    {/if}

    <fieldset class="checks">
      <legend>Data</legend>
      {#each page.data as decl (decl.span.start)}
        <div class="data-row">
          <code>{decl.name.name}: {decl.list ? `list<${decl.entity.name}>` : decl.entity.name}</code>
          <button class="small" onclick={() => onedit({ op: "remove", item: decl.span.start })}>Remove</button>
        </div>
      {:else}
        <p class="hint">No data yet. Widgets bind to the data declared here.</p>
      {/each}
      <div class="add-data">
        <input bind:value={dataName} placeholder="name" />
        <select bind:value={dataEntity}>
          <option value="" disabled>entity</option>
          {#each entities as entity (entity.name.name)}
            <option value={entity.name.name}>{entity.name.name}</option>
          {/each}
        </select>
        <label class="inline"><input type="checkbox" bind:checked={dataList} /> list</label>
        <button class="small" onclick={addData} disabled={!dataName.trim() || !dataEntity}>Add</button>
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
            <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
            <button class="small" title="Unwire" onclick={() => setOwnerHandler(ev, null)}>&times;</button>
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
  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
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
</style>
