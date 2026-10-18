// The agent panel's state: the conversation as shown, the connection it
// talks through, and whether a turn is running. Turns run in the backend
// and report through events.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { agentConnections, applyEvent, type Item } from "./agent";
import type { AgentEnded } from "./generated/AgentEnded";
import type { AgentEvent } from "./generated/AgentEvent";
import type { ConnectionRow } from "./generated/ConnectionRow";
import { unlock } from "./unlockStore.svelte";

const OPEN_KEY = "clarity.agent.open";
const CONNECTION_KEY = "clarity.agent.connection";

function remembered(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function remember(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Remembering is a convenience only.
  }
}

class AgentStore {
  open = $state(remembered(OPEN_KEY) === "true");
  items = $state<Item[]>([]);
  running = $state(false);
  connections = $state<string[]>([]);
  connection = $state(remembered(CONNECTION_KEY) ?? "anthropic");
  /** What the screen does when a turn ends; App sets it. */
  onEnded: (ended: AgentEnded) => void = () => {};

  constructor() {
    listen<AgentEvent>("agent-event", (e) => (this.items = applyEvent(this.items, e.payload)));
    listen<AgentEnded>("agent-ended", (e) => {
      this.running = false;
      this.onEnded(e.payload);
    });
  }

  toggle() {
    this.open = !this.open;
    remember(OPEN_KEY, String(this.open));
    if (this.open) this.loadConnections();
  }

  async loadConnections() {
    try {
      this.connections = agentConnections(await invoke<ConnectionRow[]>("connections_list"));
    } catch {
      this.connections = [];
    }
    if (this.connections.length > 0 && !this.connections.includes(this.connection)) this.pick(this.connections[0]);
  }

  pick(connection: string) {
    this.connection = connection;
    remember(CONNECTION_KEY, connection);
  }

  async send(text: string): Promise<boolean> {
    const said = text.trim();
    if (!said || this.running) return false;
    this.items = [...this.items, { kind: "user", text: said }];
    this.running = true;
    try {
      // The key comes from the vault: a locked vault asks for its password, then the turn starts.
      await unlock.invoke("agent_send", { text: said, connection: this.connection });
    } catch (err) {
      this.running = false;
      this.items = [...this.items, { kind: "error", text: String(err) }];
    }
    return true;
  }

  async stop() {
    await invoke("agent_stop");
  }

  async forget() {
    try {
      await invoke("agent_forget");
      this.items = [];
    } catch (err) {
      this.items = [...this.items, { kind: "error", text: String(err) }];
    }
  }
}

export const agent = new AgentStore();
