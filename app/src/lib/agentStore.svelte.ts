// The agent panel's state: the conversation as shown, the connection it
// talks through, whether a turn is running, and the tokens the turn and the
// conversation cost. Turns run in the backend
// and report through events.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { addUsage, agentConnections, outsideAgents, applyAnswer, applyAsk, applyEvent, NO_USAGE, type Item } from "./agent";
import { isRuns } from "./agent";
import type { AgentAnswered } from "@clarity/ui/generated/AgentAnswered.ts";
import type { Runs } from "@clarity/ui/generated/Runs.ts";
import type { AgentAsk } from "@clarity/ui/generated/AgentAsk.ts";
import type { AgentEnded } from "@clarity/ui/generated/AgentEnded.ts";
import type { AgentEvent } from "@clarity/ui/generated/AgentEvent.ts";
import type { ConnectionRow } from "@clarity/ui/generated/ConnectionRow.ts";
import type { Usage } from "@clarity/ui/generated/Usage.ts";
import { unlock } from "./unlockStore.svelte";

const OPEN_KEY = "clarity.agent.open";
const CONNECTION_KEY = "clarity.agent.connection";
const RUNS_KEY = "clarity.agent.runs";

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
  /** The connections that are the person's own agent program, which signs in its own way. */
  outside = $state<string[]>([]);
  /** Whether Sign in runs. */
  signingIn = $state(false);
  /** Whether the agent program said it is not signed in, so the panel offers Sign in. */
  needsSignIn = $state(false);
  connection = $state(remembered(CONNECTION_KEY) ?? "anthropic");
  /** Whether the agent asks before it runs, may run without asking, or never runs (Clarity's settings). */
  runs = $state<Runs>(((value) => (isRuns(value) ? value : "ask"))(remembered(RUNS_KEY)));
  /** The tokens of the turn that runs or ran last, and of the conversation. */
  turnUsage = $state<Usage>(NO_USAGE);
  sessionUsage = $state<Usage>(NO_USAGE);
  /** What the screen does when a turn ends; App sets it. */
  onEnded: (ended: AgentEnded) => void = () => {};

  constructor() {
    listen<AgentEvent>("agent-event", (e) => {
      const event = e.payload;
      if (event.kind === "usage") {
        this.turnUsage = addUsage(this.turnUsage, event.usage);
        this.sessionUsage = addUsage(this.sessionUsage, event.usage);
      }
      this.items = applyEvent(this.items, event);
    });
    listen<AgentAsk>("agent-ask", (e) => (this.items = applyAsk(this.items, e.payload)));
    listen<AgentAnswered>("agent-answered", (e) => (this.items = applyAnswer(this.items, e.payload)));
    listen<AgentEnded>("agent-ended", (e) => {
      this.running = false;
      this.needsSignIn = e.payload.signIn;
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
      const rows = await invoke<ConnectionRow[]>("connections_list");
      this.connections = agentConnections(rows);
      this.outside = outsideAgents(rows);
    } catch {
      this.connections = [];
      this.outside = [];
    }
    if (this.connections.length > 0 && !this.connections.includes(this.connection)) this.pick(this.connections[0]);
  }

  setRuns(runs: Runs) {
    this.runs = runs;
    remember(RUNS_KEY, runs);
  }

  pick(connection: string) {
    this.connection = connection;
    this.needsSignIn = false;
    remember(CONNECTION_KEY, connection);
  }

  async send(text: string): Promise<boolean> {
    const said = text.trim();
    if (!said || this.running) return false;
    this.items = [...this.items, { kind: "user", text: said }];
    this.turnUsage = NO_USAGE;
    this.running = true;
    try {
      // The key comes from the vault: a locked vault asks for its password, then the turn starts.
      await unlock.invoke("agent_send", { text: said, connection: this.connection, runs: this.runs });
    } catch (err) {
      this.running = false;
      this.items = [...this.items, { kind: "error", text: String(err) }];
    }
    return true;
  }

  /** Allow or Deny on the agent's question; the answer shows when the backend has it. */
  async answer(id: number, allowed: boolean) {
    try {
      await invoke("agent_answer", { id, allowed });
    } catch (err) {
      this.items = [...this.items, { kind: "error", text: String(err) }];
    }
  }

  /** Signs in to the outside agent the vendor's own way; a browser opens. */
  async signIn() {
    if (this.signingIn) return;
    this.signingIn = true;
    try {
      const said = await invoke<string>("agent_sign_in", { connection: this.connection });
      this.items = [...this.items, { kind: "note", text: said }];
      this.needsSignIn = false;
    } catch (err) {
      this.items = [...this.items, { kind: "error", text: String(err) }];
    } finally {
      this.signingIn = false;
    }
  }

  async stop() {
    await invoke("agent_stop");
  }

  async forget() {
    try {
      await invoke("agent_forget");
      this.items = [];
      this.turnUsage = NO_USAGE;
      this.sessionUsage = NO_USAGE;
    } catch (err) {
      this.items = [...this.items, { kind: "error", text: String(err) }];
    }
  }
}

export const agent = new AgentStore();
