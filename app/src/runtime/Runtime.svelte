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
  import { provideLive } from "../lib/live";
  import type { Row } from "../lib/model";
  import { findPage, type PageData } from "../lib/pages";

  /** One open window, as the backend sends it. */
  type WindowState = {
    window: string;
    data: Record<string, unknown>;
    parts: Record<string, { visible?: boolean; enabled?: boolean; caption?: string | null }>;
    selected: Record<string, number>;
  };
  type Message = { level: "info" | "warning" | "error"; text: string };
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
  let messages = $state<(Message & { id: number })[]>([]);
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
    document.title = page?.title ?? page?.name.name ?? "";
  });

  function say(message: Message) {
    const id = nextMessage++;
    messages = [...messages, { ...message, id }];
    if (message.level === "info") setTimeout(() => dismiss(id), MESSAGE_MS);
  }

  function dismiss(id: number) {
    messages = messages.filter((m) => m.id !== id);
  }

  async function open(window: string, withData: Record<string, unknown> = {}) {
    try {
      const state = await invoke<WindowState>("open_window", { window, with: withData });
      stack = [...stack, state];
    } catch (err) {
      say({ level: "error", text: String(err) });
    }
  }

  async function close() {
    stack = stack.slice(0, -1);
    if (stack.length === 0) await invoke("quit_app");
  }

  /** Runs a handler on the current window and applies the reply. */
  async function run(handler: string) {
    if (!current || busy) return;
    busy = true;
    const index = stack.length - 1;
    try {
      const reply = await invoke<Reply>("dispatch_event", { state: $state.snapshot(current), handler });
      stack[index] = reply.state;
      reply.messages.forEach(say);
      // Data may have changed: reference lists load again when needed.
      options = {};
      if (reply.close) await close();
      for (const request of reply.open) await open(request.window, request.with);
    } catch (err) {
      say({ level: "error", text: String(err) });
    } finally {
      busy = false;
    }
  }

  function navigate(target: string) {
    if (target === "close") close();
    else if (target === "quit") invoke("quit_app");
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
    refOptions(entity: string) {
      const known = options[entity];
      if (!known && !loading.has(entity)) {
        loading.add(entity);
        invoke<{ id: number; text: string }[]>("ref_options", { entity })
          .then((list) => (options[entity] = list))
          .catch((err) => say({ level: "error", text: String(err) }))
          .finally(() => loading.delete(entity));
      }
      return known ?? [];
    },
  });

  invoke<{ model: Preview; main: string }>("start").then((start) => {
    model = start.model;
    open(start.main);
  });
</script>

<div class="runtime" class:busy>
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
