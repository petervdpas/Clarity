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
  import { errorText, shownText, themeArea, themeOf, tr, type Row } from "../lib/model";
  import { languageChoice } from "../lib/i18n.svelte";
  import { languageName, languagesOf, rememberedChoice } from "../lib/translate";
  import type { ShownText } from "../lib/format";
  import { findPage, type PageData } from "../lib/pages";

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
    document.title = page?.title && model ? tr(model, page.title) : (page?.name.name ?? "");
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

  /** Shows what a handler asked for: messages, and windows to open. */
  async function follow(reply: Reply) {
    reply.messages.forEach(say);
    // Data may have changed: reference lists load again when needed.
    options = {};
    for (const request of reply.open) await open(request.window, request.with);
  }

  /** Opens a window; the backend loads its data and runs its `on open`. */
  async function open(window: string, withData: Record<string, unknown> = {}) {
    try {
      const reply = await invoke<Reply>("open_window", { window, with: withData });
      stack = [...stack, reply.state];
      if (reply.close) return close();
      await follow(reply);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    }
  }

  /** Closes the top window after its `on close`, then runs `on return` on the one below. */
  async function close() {
    const top = stack.at(-1);
    if (!top) return;
    try {
      const closing = await invoke<Reply>("close_window", { state: $state.snapshot(top) });
      closing.messages.forEach(say);
      stack = stack.slice(0, -1);
      const below = stack.at(-1);
      if (!below) {
        await invoke("quit_app");
        return;
      }
      const back = await invoke<Reply>("return_to", { state: $state.snapshot(below) });
      stack[stack.length - 1] = back.state;
      if (back.close) return close();
      await follow(back);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    }
  }

  /** Runs a handler on the current window and applies the reply. */
  async function run(handler: string) {
    if (!current || busy) return;
    busy = true;
    const index = stack.length - 1;
    try {
      const reply = await invoke<Reply>("dispatch_event", { state: $state.snapshot(current), handler });
      stack[index] = reply.state;
      if (reply.close) {
        reply.messages.forEach(say);
        await close();
        for (const request of reply.open) await open(request.window, request.with);
      } else {
        await follow(reply);
      }
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    } finally {
      busy = false;
    }
  }

  function navigate(target: string) {
    if (target === "close") close();
    else if (target === "quit") invoke("quit_app");
    else if (target === "language") choosingLanguage = true;
    else open(target);
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

  // The language the person chose with a `language` click target, kept for
  // the next start.
  const LANGUAGE_KEY = "clarity-language";
  let choosingLanguage = $state(false);
  const base = $derived(model?.project.locale.language ?? "en");
  const languages = $derived(model ? languagesOf(base, Object.keys(model.project.translations)) : []);
  // The project's theme on the whole application.
  const area = $derived(themeArea(model ? themeOf(model) : null));

  function remember(preview: Preview) {
    try {
      const stored = localStorage.getItem(LANGUAGE_KEY);
      languageChoice.language = rememberedChoice(stored, preview.project.locale.language, Object.keys(preview.project.translations));
    } catch {
      // Without storage the application follows the locale.
    }
  }

  function chooseLanguage(language: string) {
    languageChoice.language = language === base ? null : language;
    choosingLanguage = false;
    try {
      localStorage.setItem(LANGUAGE_KEY, language);
    } catch {
      // The choice holds for this run.
    }
  }

  // The app's `on start` runs before its main window opens; windows it opens
  // come on top of the main window.
  invoke<{ model: Preview; main: string }>("start").then(async (start) => {
    model = start.model;
    remember(start.model);
    try {
      const started = await invoke<Reply>("start_app");
      await open(start.main);
      await follow(started);
    } catch (err) {
      say({ level: "error", text: errorText(err) });
    }
  });
</script>

<div class="runtime {area.class ?? ''}" class:busy data-scheme={area.scheme}>
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

  {#if choosingLanguage}
    <div class="language-backdrop" role="presentation" onclick={() => (choosingLanguage = false)}>
      <div
        class="language-picker"
        role="dialog"
        aria-label="Language"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.key === "Escape" && (choosingLanguage = false)}
      >
        {#each languages as language (language)}
          <button
            class:current={(languageChoice.language ?? base) === language}
            onclick={() => chooseLanguage(language)}
            lang={language}>{languageName(language)}</button
          >
        {/each}
      </div>
    </div>
  {/if}

  <div class="messages" aria-live="polite">
    {#each messages as m (m.id)}
      <button class="message {m.level}" onclick={() => dismiss(m.id)} title="Dismiss">{m.text}</button>
    {/each}
  </div>
</div>

<style>
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

  .language-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.25);
    display: grid;
    place-items: center;
    z-index: 20;
  }

  .language-picker {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 200px;
  }

  .language-picker button.current {
    border-color: var(--accent);
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
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    background: var(--panel);
    color: var(--text);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.3);
  }

  .message.warning {
    border-color: var(--warning, #d9a400);
  }

  .message.error {
    border-color: var(--error);
    color: var(--error);
  }
</style>
