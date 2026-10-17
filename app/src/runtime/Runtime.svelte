<script lang="ts">
  // A running application: the windows the backend opens, drawn by the
  // renderer, with every event sent to the backend. The backend owns the
  // behavior; this page only shows state and reports what the user does.

  import { invoke } from "@tauri-apps/api/core";
  import FrameView from "../lib/FrameView.svelte";
  import GridBox from "../lib/GridBox.svelte";
  import { frameForPage } from "../lib/frame";
  import type { Event as PartEvent } from "../lib/generated/Event";
  import type { Preview } from "../lib/generated/Preview";
  import { provideLive, type PartState } from "../lib/live";
  import { errorText, offeredThemes, shownText, themeArea, themeOf, tr, type Row } from "../lib/model";
  import ChoicePicker from "../lib/ChoicePicker.svelte";
  import { themeChoice } from "../lib/scheme.svelte";
  import { languageChoice } from "../lib/i18n.svelte";
  import { languageName, languagesOf, rememberedChoice } from "../lib/translate";
  import type { ShownText } from "../lib/format";
  import { findPage, type PageData } from "../lib/pages";
  import { Navigator } from "../lib/navigator";

  /** One open window, as the backend sends it. */
  type WindowState = {
    window: string;
    data: Record<string, unknown>;
    parts: Record<string, PartState>;
    selected: Record<string, number>;
  };
  type Message = { level: "info" | "warning" | "error"; text: ShownText };
  type Reply = {
    state: WindowState;
    messages: Message[];
    open: { window: string; with: Record<string, unknown> }[];
    close: boolean;
  };

  /** How long an information message stays. */
  const MESSAGE_MS = 5000;

  let model = $state<Preview | null>(null);
  let stack = $state<WindowState[]>([]);
  /** The messages on screen, their text filled. */
  let messages = $state<{ level: Message["level"]; text: string; id: number }[]>([]);
  let busy = $state(false);
  let options = $state<Record<string, { id: number; text: string }[]>>({});
  let nextMessage = 0;

  const current = $derived(stack.at(-1) ?? null);
  const page = $derived(model && current ? findPage(model, current.window) : undefined);

  // The window's data as the renderer reads it; the rows are the state's own
  // records, so what the user types is in the state the next event sends.
  const data = $derived.by(() => {
    const out: PageData = {};
    if (!model || !page || !current) return out;
    for (const decl of page.data) {
      const entity = model.project.files.flatMap((f) => f.entities).find((e) => e.name.name === decl.entity.name);
      const value = current.data[decl.name.name];
      if (!entity || value === undefined) continue;
      const rows = (decl.list ? value : [value]) as Row[];
      out[decl.name.name] = { entity, list: decl.list, rows };
    }
    return out;
  });

  $effect(() => {
    const title = page?.title && model ? tr(model, page.title) : (page?.name.name ?? "");
    document.title = title;
    // The desktop window shows it too; the designer's Preview has no such command.
    invoke("set_title", { title }).catch(() => {});
  });

  function say(message: Message) {
    const id = nextMessage++;
    const text = model ? shownText(model, message.text) : typeof message.text === "string" ? message.text : message.text.text;
    messages = [...messages, { level: message.level, text, id }];
    if (message.level === "info") setTimeout(() => dismiss(id), MESSAGE_MS);
  }

  function dismiss(id: number) {
    messages = messages.filter((m) => m.id !== id);
  }

  /** Set when this is a desktop window of its own (`in new window`), not the main one. */
  let apart = false;

  // Opening and closing windows, as on every stack (lib/navigator.ts). The
  // backend gets snapshots, so what the user types is in the state it sees.
  const nav = new Navigator<WindowState, Message>(
    {
      get: () => stack,
      set: (next) => {
        stack = next;
        // Data may have changed: reference lists load again when needed.
        options = {};
      },
    },
    {
      open: (window, withData) => invoke<Reply>("open_window", { window, with: withData }),
      close: (state) => invoke<Reply>("close_window", { state: $state.snapshot(state) }),
      returnTo: (state) => invoke<Reply>("return_to", { state: $state.snapshot(state) }),
      openApart: (window, withData) => invoke("open_apart", { window, with: withData }),
      // The main window ends the application; a window of its own just closes.
      quit: () => invoke(apart ? "close_apart" : "quit_app"),
    },
    say,
    (err) => say({ level: "error", text: errorText(err) }),
  );

  /** Runs a handler on the current window and applies the reply. */
  async function run(handler: string) {
    if (!current || busy) return;
    busy = true;
    const index = stack.length - 1;
    try {
      const reply = await invoke<Reply>("dispatch_event", { state: $state.snapshot(current), handler });
      await nav.apply(index, reply);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
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

  provideLive({
    get parts() {
      return current?.parts ?? {};
    },
    get selected() {
      return current?.selected ?? {};
    },
    event(_part: string, _event: PartEvent, handler: string) {
      run(handler);
    },
    select(part: string, row: number) {
      if (current) current.selected[part] = row;
    },
    setText(part: string, text: string) {
      if (current) current.parts[part] = { ...current.parts[part], text };
    },
    refOptions(entity: string) {
      const known = options[entity];
      if (!known && !loading.has(entity)) {
        loading.add(entity);
        invoke<{ id: number; text: string }[]>("ref_options", { entity })
          .then((list) => (options[entity] = list))
          .catch((err) => say({ level: "error", text: errorText(err) }))
          .finally(() => loading.delete(entity));
      }
      return known ?? [];
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
  }

  function chooseTheme(theme: string) {
    themeChoice.theme = theme;
    choosing = null;
    store(THEME_KEY, theme);
  }

  // The app's `on start` runs before its main window opens; windows it opens
  // come on top of the main window.
  invoke<{ model: Preview; main: string }>("start").then(async (start) => {
    model = start.model;
    remember(start.model);
    try {
      // A desktop window of its own starts with the window it was opened for.
      const own = await invoke<{ window: string; with: Record<string, unknown> } | null>("apart_start").catch(() => null);
      if (own) {
        apart = true;
        await nav.open(own.window, own.with);
        return;
      }
      const started = await invoke<Reply>("start_app");
      await nav.open(start.main);
      await nav.follow(started);
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
      title="Language"
      choices={languages.map((l) => ({ value: l, label: languageName(l), lang: l }))}
      current={languageChoice.language ?? base}
      onchoose={chooseLanguage}
      onclose={() => (choosing = null)}
    />
  {:else if choosing === "theme" && model}
    <ChoicePicker
      title="Theme"
      choices={offeredThemes(model).map((t) => ({ value: t.name.name, label: tr(model!, t.label) }))}
      current={themeOf(model)?.name.name}
      onchoose={chooseTheme}
      onclose={() => (choosing = null)}
    />
  {/if}

  <div class="messages" aria-live="polite">
    {#each messages as m (m.id)}
      <button class="message {m.level}" onclick={() => dismiss(m.id)} title="Dismiss">{m.text}</button>
    {/each}
  </div>
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
    text-align: left;
    padding: 10px 14px;
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
</style>
