<script lang="ts">
  import type { RefOption } from "../lib/dropdownOptions";
  // A running application: the windows the backend opens, drawn by the
  // renderer, with every event sent to the backend. The backend owns the
  // behavior; this page only shows state and reports what the user does.

  import { invoke } from "../lib/backend";
  import FrameView from "../lib/FrameView.svelte";
  import GridBox from "../lib/GridBox.svelte";
  import { frameForPage } from "../lib/frame";
  import type { Event as PartEvent } from "../lib/generated/Event";
  import type { Preview } from "../lib/generated/Preview";
  import { provideLive, type PartState, type Pointer } from "../lib/live";
  import { errorText, languageOf, offeredThemes, ownWord, shownText, themeArea, themeOf, tr, type Row } from "../lib/model";
  import ChoicePicker from "../lib/ChoicePicker.svelte";
  import { themeChoice } from "../lib/scheme.svelte";
  import { languageChoice } from "../lib/i18n.svelte";
  import { languageName, languagesOf, rememberedChoice } from "../lib/translate";
  import type { ShownText } from "../lib/format";
  import { allParts, findPage, valueEntry, windowSize, type PageData } from "../lib/pages";
  import { handlerCall } from "../lib/calls";
  import type { Place } from "../lib/customParts";
  import { dataEntity, isList, translatedDefaults } from "../lib/windowData";
  import { findApp } from "../lib/frame";
  import { Navigator } from "../lib/navigator";
  import AskDialog from "../lib/ui/AskDialog.svelte";
  import Dialog from "../lib/ui/Dialog.svelte";
  import { answerValues, type Ask } from "../lib/asks";
  import { keptSettings, settingsDefaults, settingsEntity, settingsOf } from "../lib/settings";
  import type { JsonValue } from "../lib/generated/serde_json/JsonValue";

  /** One open window, as the backend sends it. */
  type WindowState = {
    window: string;
    /** The number of this open window, its own. */
    instance?: number;
    data: Record<string, unknown>;
    parts: Record<string, PartState>;
    selected: Record<string, number>;
  };
  type Message = { level: "info" | "warning" | "error"; text: ShownText };
  type Reply = {
    state: WindowState;
    messages: Message[];
    /** The texts handlers put in text values, which show in the person's language and format. */
    shown?: Record<string, ShownText>;
    open: { window: string; with: Record<string, unknown> }[];
    close: boolean;
    /** The questions its handler asked, asked when it has finished (DESIGN 5.14). */
    asks?: Ask[];
    /** The hears its handler gave custom parts, in order (DESIGN 5.19). */
    hears?: { part: string; hear: string; values: JsonValue[] }[];
  };

  /**
   * What waits to be shown in Clarity's dialog, one at a time: a question,
   * with the place of the window that asked; or a warning or an error,
   * read and closed with OK.
   */
  type Waiting = { kind: "ask"; ask: Ask; window: number } | { kind: "message"; level: Message["level"]; text: string };

  /** How long an information message stays. */
  const MESSAGE_MS = 5000;

  let model = $state<Preview | null>(null);
  let stack = $state<WindowState[]>([]);
  /** The messages on screen, their text filled. */
  let messages = $state<{ level: Message["level"]; text: string; id: number }[]>([]);
  /** The dialogs waiting, the first one shown. */
  let waiting = $state<Waiting[]>([]);
  let busy = $state(false);
  let options = $state<Record<string, RefOption[]>>({});
  /** The values each typed dropdown's field has, by `Entity.field`. */
  let fieldValues = $state<Record<string, string[]>>({});
  let nextMessage = 0;

  const current = $derived(stack.at(-1) ?? null);
  const page = $derived(model && current ? findPage(model, current.window) : undefined);

  // The window's data as the renderer reads it; the rows are the state's own
  // records, so what the user types is in the state the next event sends.
  const data = $derived.by(() => {
    const out: PageData = {};
    if (!model || !page || !current) return out;
    for (const decl of page.data) {
      const value = current.data[decl.name.name];
      if (value === undefined) continue;
      // A plain value is read and written in the window's data itself.
      if (decl.of.kind === "value") {
        const entry = valueEntry(decl, current.data as Row);
        if (entry) out[decl.name.name] = entry;
        continue;
      }
      const entity = model.project.files.flatMap((f) => f.entities).find((e) => e.name.name === dataEntity(decl));
      if (!entity) continue;
      const rows = (isList(decl) ? value : [value]) as Row[];
      out[decl.name.name] = { entity, list: isList(decl), rows };
    }
    return out;
  });

  $effect(() => {
    const title = page?.title && model ? tr(model, page.title) : (page?.name.name ?? "");
    document.title = title;
    // The desktop window shows it too; the designer's Preview has no such command.
    invoke("set_title", { title }).catch(() => {});
  });

  /** Shows a message: information as a note that goes away by itself, a warning or an error in Clarity's dialog. */
  function say(message: Message) {
    const raw = message.text;
    const text = model ? shownText(model, raw) : typeof raw === "string" ? raw : "key" in raw ? raw.key : raw.text;
    if (message.level !== "info") {
      waiting = [...waiting, { kind: "message", level: message.level, text }];
      return;
    }
    const id = nextMessage++;
    messages = [...messages, { level: message.level, text, id }];
    setTimeout(() => dismiss(id), MESSAGE_MS);
  }

  /** The dialog on screen is done with. */
  function shown() {
    waiting = waiting.slice(1);
  }

  /** The answer to a question: runs the handler it goes to on the window that asked, if that is still open. */
  async function answer(item: Extract<Waiting, { kind: "ask" }>, value: JsonValue | null) {
    const state = stack[item.window];
    if (!state) return;
    busy = true;
    try {
      const values = answerValues(item.ask, value);
      const reply = received(await invoke<Reply>("answer_event", { state: $state.snapshot(state), handler: item.ask.handler, values }));
      await nav.apply(item.window, reply);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    } finally {
      busy = false;
    }
  }

  function dismiss(id: number) {
    messages = messages.filter((m) => m.id !== id);
  }

  /** Set when this is a desktop window of its own (`in new window`), not the main one. */
  let apart = false;

  /** A reply with the texts its handlers put in text values filled in the person's language and format. */
  function received(reply: Reply, start = false): Reply {
    // A handler may have set the settings; the app's start has only their defaults.
    if (!start) adopt(reply.state.data[SETTINGS]);
    // The hears go to the custom parts of the window the handler ran on.
    for (const h of reply.hears ?? []) {
      invoke("hear_part", { instance: reply.state.instance ?? 0, part: h.part, hear: h.hear, values: h.values }).catch(() => {});
    }
    if (!model || !reply.shown) return reply;
    const data = { ...reply.state.data };
    for (const [name, text] of Object.entries(reply.shown)) data[name] = shownText(model, text);
    return { ...reply, shown: undefined, state: { ...reply.state, data } };
  }

  /** A window as it opens: its text values still at their defaults in the person's language. */
  function opened(reply: Reply): Reply {
    const target = model ? findPage(model, reply.state.window) : undefined;
    if (!model || !target) return received(reply);
    const decls = [...target.data, ...(findApp(model)?.data ?? [])];
    return received({ ...reply, state: { ...reply.state, data: translatedDefaults(decls, reply.state.data, (text) => tr(model!, text)) } });
  }

  // The person's settings (DESIGN 5.17), when the project has them: what the
  // shell kept, over their defaults. Every open window has them as its data
  // `settings`; a change from an input, a handler or a picker is kept by the
  // shell and switches the language and the theme.
  const SETTINGS = "settings";
  let settings: Record<string, unknown> | null = null;

  /** The settings when the application starts: what was kept and still fits, over the defaults. */
  async function startSettings(preview: Preview) {
    if (!settingsEntity(preview)) return;
    const kept = await invoke<Record<string, unknown> | null>("read_settings").catch(() => null);
    const themes = offeredThemes(preview).map((t) => t.name.name);
    const language = languageChoice.language ?? base;
    const theme = themeOf(preview)?.name.name ?? themes[0] ?? "";
    settings = { ...settingsDefaults(preview, language, theme), ...keptSettings(settingsOf(preview, languages, themes), kept ?? {}) };
    apply(settings);
  }

  /** The data a window opens with: its settings as they are now. */
  function withSettings(withData: Record<string, unknown>): Record<string, unknown> {
    return settings ? { ...withData, [SETTINGS]: { ...settings } } : withData;
  }

  /** The settings changed: every open window has them, the shell keeps them, the language and theme follow. */
  function adopt(next: unknown) {
    if (!settings || !next || typeof next !== "object") return;
    const values = next as Record<string, unknown>;
    if (JSON.stringify(values) === JSON.stringify(settings)) return;
    settings = { ...values };
    for (const state of stack) if (SETTINGS in state.data) state.data[SETTINGS] = { ...settings };
    const { id: _id, ...kept } = settings;
    invoke("keep_settings", { settings: kept }).catch((err) => say({ level: "error", text: errorText(err) }));
    apply(settings);
  }

  /** Shows the language and theme the settings name, when they have them. */
  function apply(values: Record<string, unknown>) {
    if (typeof values.language === "string" && values.language !== "" && values.language !== (languageChoice.language ?? base)) {
      languageChoice.language = values.language === base ? null : values.language;
      store(LANGUAGE_KEY, values.language);
      void tellLanguage();
    }
    if (typeof values.theme === "string" && values.theme !== "" && values.theme !== themeChoice.theme) {
      themeChoice.theme = values.theme;
      store(THEME_KEY, values.theme);
    }
  }

  /** Tells the backend the language the screen picked, so the text code makes is in it too. */
  async function tellLanguage() {
    if (!model) return;
    const language = languageOf(model);
    await invoke("set_language", { language }).catch(() => {});
  }

  // Opening and closing windows, as on every stack (lib/navigator.ts). The
  // backend gets snapshots, so what the user types is in the state it sees.
  const nav = new Navigator<WindowState, Message, Ask>(
    {
      get: () => stack,
      set: (next) => {
        // A window that closed ends its custom parts.
        const open = new Set(next.map((s) => s.instance));
        for (const gone of stack.filter((s) => s.instance !== undefined && !open.has(s.instance))) {
          invoke("close_parts", { instance: gone.instance }).catch(() => {});
        }
        stack = next;
        // Data may have changed: reference lists and values load again when needed.
        options = {};
        fieldValues = {};
      },
    },
    {
      open: async (window, withData) => opened(await invoke<Reply>("open_window", { window, with: withSettings(withData) })),
      close: async (state) => received(await invoke<Reply>("close_window", { state: $state.snapshot(state) })),
      returnTo: async (state) => received(await invoke<Reply>("return_to", { state: $state.snapshot(state) })),
      // A desktop window of its own opens at the size of the window it shows.
      openApart: (window, withData) => {
        const target = model ? findPage(model, window) : undefined;
        const size = target ? windowSize(target) : undefined;
        return invoke("open_apart", { window, with: withSettings(withData), width: size?.width, height: size?.height });
      },
      // The main window ends the application; a window of its own just closes.
      quit: () => invoke(apart ? "close_apart" : "quit_app"),
    },
    say,
    (err) => say({ level: "error", text: errorText(err) }),
    // A question waits its turn; one the app's own start asks has no window to answer it.
    // A dialog window asked for opens at once, as `open` opens one.
    (ask, window) => {
      if (window < 0) return;
      if (ask.form.kind === "window") void openAsked(ask, ask.form.window, ask.form.with, window);
      else waiting = [...waiting, { kind: "ask", ask, window }];
    },
    // What a dialog asked for closed with runs the handler of the window that asked.
    (state, result) => {
      const asked = state.instance === undefined ? undefined : dialogsAsked.get(state.instance);
      if (!asked) return;
      dialogsAsked.delete(state.instance!);
      void answer({ kind: "ask", ask: asked.ask, window: asked.window }, result as JsonValue);
    },
  );

  /** The dialogs a question opened, by the number of their open window: the question, and the place of the window that asked. */
  const dialogsAsked = new Map<number, { ask: Ask; window: number }>();

  async function openAsked(ask: Ask, dialog: string, withData: Record<string, JsonValue>, window: number) {
    await nav.open(dialog, withData);
    const opened = stack.at(-1);
    if (opened?.window === dialog && opened.instance !== undefined) dialogsAsked.set(opened.instance, { ask, window });
  }

  /** Runs a handler on the current window and applies the reply; false when it refused or failed. */
  async function run(handler: string): Promise<boolean> {
    if (!current || busy) return false;
    busy = true;
    const index = stack.length - 1;
    try {
      const reply = received(await invoke<Reply>("dispatch_event", { state: $state.snapshot(current), handler }));
      await nav.apply(index, reply);
      return true;
    } catch (err) {
      say({ level: "error", text: errorText(err) });
      return false;
    } finally {
      busy = false;
    }
  }

  // A custom part told something (DESIGN 5.19): the shell calls this with the
  // window's number, the part, the tell and its values. `change` is the value
  // the person changed, which goes into what the part binds and runs `on
  // change`; another tell runs the handler wired to it with its values. One
  // at a time, after what runs now.
  let telling: Promise<void> = Promise.resolve();
  (globalThis as unknown as { clarityTold: (instance: number, part: string, tell: string, values: JsonValue[]) => void }).clarityTold = (instance, part, tell, values) => {
    telling = telling.then(() => told(instance, part, tell, values)).catch((err) => say({ level: "error", text: errorText(err) }));
  };

  async function told(instance: number, part: string, tell: string, values: JsonValue[]) {
    while (busy) await new Promise((done) => setTimeout(done, 5));
    const index = stack.findIndex((s) => s.instance === instance);
    const state = stack[index];
    const target = model && state ? findPage(model, state.window) : undefined;
    const widget = target ? allParts(target).find((w) => w.props.name === part) : undefined;
    if (!state || !widget) return;
    if (tell === "change") {
      const [data, field] = (widget.props.bind ?? "").split(".");
      const record = state.data[data] as Record<string, unknown> | undefined;
      if (field && record && typeof record === "object") record[field] = values[0];
      else if (data && !field) state.data[data] = values[0];
      const wired = widget.props.handlers.find((h) => h.event === "change");
      if (wired && index === stack.length - 1) await run(handlerCall(wired));
      return;
    }
    const wired = widget.props.tells.find((t) => t.tell === tell);
    if (!wired) return;
    busy = true;
    try {
      const reply = received(await invoke<Reply>("answer_event", { state: $state.snapshot(state), handler: wired.handler, values }));
      await nav.apply(index, reply);
    } finally {
      busy = false;
    }
  }

  // A window opened elsewhere (`in new window`) has closed: this one is on top again.
  window.addEventListener("clarity-return", () => nav.returned());

  function navigate(target: string, newWindow = false) {
    if (target === "close") nav.close();
    else if (target === "quit") invoke("quit_app");
    else if (target === "language") choosing = "language";
    else if (target === "theme") choosing = "theme";
    else nav.target(target, newWindow);
  }

  const loading = new Set<string>();
  // The stored pictures fetched so far, by entity, record and field.
  let pictures = $state<Record<string, string | null>>({});
  const fetching = new Set<string>();
  const pictureKey = (entity: string, id: number, field: string) => `${entity}/${id}/${field}`;

  provideLive({
    get parts() {
      return current?.parts ?? {};
    },
    get selected() {
      return current?.selected ?? {};
    },
    event(_part: string, _event: PartEvent, handler: string) {
      return run(handler);
    },
    select(part: string, row: number) {
      if (current) current.selected[part] = row;
    },
    setText(part: string, text: string) {
      if (current) current.parts[part] = { ...current.parts[part], text };
    },
    setChecked(part: string, checked: boolean) {
      if (current) current.parts[part] = { ...current.parts[part], checked };
    },
    setPointer(part: string, pointer: Pointer) {
      if (current) current.parts[part] = { ...current.parts[part], pointer };
    },
    setFolded(part: string, folded: boolean) {
      if (current) current.parts[part] = { ...current.parts[part], folded };
    },
    setSidebar(part: string, change: { collapsed?: boolean; maximized?: boolean }) {
      if (current) current.parts[part] = { ...current.parts[part], ...change };
    },
    setRibbon(part: string, change: { tab?: number; collapsed?: boolean }) {
      if (current) current.parts[part] = { ...current.parts[part], ...change };
    },
    setStep(part: string, step: number) {
      if (current) current.parts[part] = { ...current.parts[part], step };
    },
    setOpen(part: string, open: number) {
      if (current) current.parts[part] = { ...current.parts[part], open };
    },
    setVisible(part: string, visible: boolean) {
      if (current) current.parts[part] = { ...current.parts[part], visible };
    },
    keepSettings() {
      if (current) adopt($state.snapshot(current.data[SETTINGS]));
    },
    placePart(part: string, kind: string, place: Place, shown: boolean) {
      if (current?.instance === undefined) return;
      invoke("place_part", { instance: current.instance, part, kind, ...place, shown }).catch(() => {});
    },
    setPartValue(part: string, name: string, value: JsonValue) {
      if (current?.instance === undefined) return;
      invoke("set_part", { instance: current.instance, part, name, value }).catch(() => {});
    },
    fieldValues(entity: string, field: string) {
      const at = `${entity}.${field}`;
      const known = fieldValues[at];
      if (!known && !loading.has(at)) {
        loading.add(at);
        invoke<string[]>("field_values", { entity, field })
          .then((list) => (fieldValues[at] = list))
          .catch((err) => say({ level: "error", text: errorText(err) }))
          .finally(() => loading.delete(at));
      }
      return known ?? [];
    },
    refOptions(entity: string) {
      const known = options[entity];
      if (!known && !loading.has(entity)) {
        loading.add(entity);
        invoke<RefOption[]>("ref_options", { entity })
          .then((list) => (options[entity] = list))
          .catch((err) => say({ level: "error", text: errorText(err) }))
          .finally(() => loading.delete(entity));
      }
      return known ?? [];
    },
    picture(entity: string, id: number, field: string) {
      const key = pictureKey(entity, id, field);
      if (key in pictures) return pictures[key];
      if (!fetching.has(key)) {
        fetching.add(key);
        invoke<string | null>("picture", { entity, id, field })
          .then((url) => (pictures[key] = url))
          .catch((err) => {
            pictures[key] = null;
            say({ level: "error", text: errorText(err) });
          })
          .finally(() => fetching.delete(key));
      }
      return undefined;
    },
    storeFile(store: string, path: string) {
      const key = `file:${store}/${path}`;
      if (key in pictures) return pictures[key];
      if (!fetching.has(key)) {
        fetching.add(key);
        invoke<string>("store_file", { store, path })
          .then((url) => (pictures[key] = url))
          .catch((err) => {
            pictures[key] = null;
            say({ level: "error", text: errorText(err) });
          })
          .finally(() => fetching.delete(key));
      }
      return undefined;
    },
    fileData(entity: string, id: number, field: string) {
      return invoke<string | null>("file_data", { entity, id, field }).catch((err) => {
        say({ level: "error", text: errorText(err) });
        return null;
      });
    },
    openFile(name: string, data: string) {
      return invoke<void>("open_file", { name, data }).catch((err) => say({ level: "error", text: errorText(err) }));
    },
    saveFile(name: string, data: string) {
      return invoke<string>("save_file", { name, data })
        .then(() => say({ level: "info", text: model ? ownWord(model, "A copy is in your Downloads folder") : "A copy is in your Downloads folder" }))
        .catch((err) => say({ level: "error", text: errorText(err) }));
    },
    forgetPicture(entity: string, id: number, field: string) {
      delete pictures[pictureKey(entity, id, field)];
    },
  });

  // The language and theme the person chose with the `language` and `theme`
  // click targets, kept for the next start.
  const LANGUAGE_KEY = "clarity-language";
  const THEME_KEY = "clarity-theme";
  let choosing = $state<"language" | "theme" | null>(null);
  const base = $derived(model?.project.locale.language ?? "en");
  const languages = $derived(model ? languagesOf(base, Object.keys(model.project.translations)) : []);
  // The theme shown, on the whole application.
  const area = $derived(themeArea(model ? themeOf(model) : null));

  function stored(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch {
      // Without storage the application starts as designed.
      return null;
    }
  }

  function store(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // The choice holds for this run.
    }
  }

  function remember(preview: Preview) {
    languageChoice.language = rememberedChoice(stored(LANGUAGE_KEY), preview.project.locale.language, Object.keys(preview.project.translations));
    // pickTheme falls back to the first theme when this one is gone.
    themeChoice.theme = stored(THEME_KEY) ?? undefined;
  }

  function chooseLanguage(language: string) {
    languageChoice.language = language === base ? null : language;
    choosing = null;
    store(LANGUAGE_KEY, language);
    void tellLanguage();
    // The picker changes the same setting a settings window shows.
    if (settings && "language" in settings) adopt({ ...settings, language });
  }

  function chooseTheme(theme: string) {
    themeChoice.theme = theme;
    choosing = null;
    store(THEME_KEY, theme);
    if (settings && "theme" in settings) adopt({ ...settings, theme });
  }

  // The app's `on start` runs before its main window opens; windows it opens
  // come on top of the main window.
  invoke<{ model: Preview; main: string }>("start").then(async (start) => {
    model = start.model;
    remember(start.model);
    await startSettings(start.model);
    await tellLanguage();
    try {
      // A desktop window of its own starts with the window it was opened for.
      const own = await invoke<{ window: string; with: Record<string, unknown> } | null>("apart_start").catch(() => null);
      if (own) {
        apart = true;
        await nav.open(own.window, own.with);
        return;
      }
      const started = received(await invoke<Reply>("start_app"), true);
      await nav.open(start.main);
      await nav.follow(started, -1);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    }
  });
</script>

<div class="runtime {area.class}" class:busy data-scheme={area.scheme}>
  {#if model && page && current}
    {#key stack.length}
      <FrameView preview={model} {page} frame={frameForPage(model, page)} {data} design={false} onnavigate={navigate}>
        <div class="client">
          <GridBox
            preview={model}
            {page}
            {data}
            design={false}
            widgets={page.widgets}
            columns={page.columns}
            container={page.span.start}
            onnavigate={navigate}
          />
        </div>
      </FrameView>
    {/key}
  {:else if model && current}
    <p class="missing">This application has no window {current.window}.</p>
  {/if}

  {#if choosing === "language"}
    <ChoicePicker
      title={model ? ownWord(model, "Language") : "Language"}
      choices={languages.map((l) => ({ value: l, label: languageName(l), lang: l }))}
      current={languageChoice.language ?? base}
      onchoose={chooseLanguage}
      onclose={() => (choosing = null)}
    />
  {:else if choosing === "theme" && model}
    <ChoicePicker
      title={ownWord(model, "Theme")}
      choices={offeredThemes(model).map((t) => ({ value: t.name.name, label: tr(model!, t.label) }))}
      current={themeOf(model)?.name.name}
      onchoose={chooseTheme}
      onclose={() => (choosing = null)}
    />
  {/if}

  <div class="messages" aria-live="polite">
    {#each messages as m (m.id)}
      {@const close = model ? ownWord(model, "Dismiss") : "Dismiss"}
      <div class="message {m.level}">
        <span>{m.text}</span>
        <button class="close" onclick={() => dismiss(m.id)} title={close} aria-label={close}>×</button>
      </div>
    {/each}
  </div>

  {#if model && waiting.length > 0}
    {@const first = waiting[0]}
    {#if first.kind === "ask"}
      {#key first}
        <AskDialog
          {model}
          ask={first.ask}
          options={(entity) => invoke("ref_options", { entity })}
          onanswer={(value) => {
            shown();
            void answer(first, value);
          }}
          oncancel={shown}
        />
      {/key}
    {:else}
      <Dialog label={first.text} onclose={shown}>
        <p class="said {first.level}">{first.text}</p>
        <div class="buttons">
          <!-- svelte-ignore a11y_autofocus -->
          <button class="ok" autofocus onclick={shown}>{ownWord(model, "OK")}</button>
        </div>
      </Dialog>
    {/if}
  {/if}
</div>

<style>
  /* The application fills its window; its look is the theme's (render.css). */
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    height: 100%;
  }

  .runtime {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .runtime.busy {
    cursor: progress;
  }

  .runtime > :global(*:first-child) {
    flex: 1;
  }

  .client {
    padding: 12px;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }

  .missing {
    padding: 24px;
    color: var(--error);
  }

  .messages {
    position: fixed;
    top: 12px;
    right: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(420px, calc(100vw - 24px));
    z-index: 100;
  }

  .message {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    text-align: left;
    padding: 10px 10px 10px 14px;
    border-radius: var(--radius-panel);
    border: var(--border-width) solid var(--border-strong);
    background: var(--panel);
    color: var(--text);
    box-shadow: var(--shadow);
  }

  .message.warning {
    border-color: var(--warning);
  }

  .message.error {
    border-color: var(--error);
    color: var(--error);
  }

  .message span {
    flex: 1;
  }

  /* The small x that closes a note before it goes by itself. */
  .message .close {
    flex: none;
    padding: 0 4px;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }

  .message .close:hover {
    color: var(--text);
  }

  .said {
    margin: 0;
    line-height: 1.5;
  }

  .said.error {
    color: var(--error);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
  }
</style>
