// The Vault tab's state: Clarity's own vault of secrets and its
// connections, which Clarity uses while you design. The ribbon, the sidebar
// and the main area all work on this one store.

import type { ConnectionRow } from "./generated/ConnectionRow";
import type { FieldValue } from "./generated/FieldValue";
import type { SecretName } from "./generated/SecretName";
import type { SecretRow } from "./generated/SecretRow";
import type { VaultInfo } from "./generated/VaultInfo";
import { LOCKED } from "./unlock";
import { unlock } from "./unlockStore.svelte";
import { CategoryEditor, type CategoryRename, connectionProblems, freshName, shownSecrets } from "./vault";


/** A secret being added or changed; `was` is the one it changes. */
export type SecretDraft = { was: SecretName | null; category: string; key: string; value: string; description: string };

class VaultStore {
  view = $state<"secrets" | "connections">("secrets");
  info = $state<VaultInfo | null>(null);
  rows = $state<SecretRow[]>([]);
  categories = $state<string[]>([]);
  /** The category picked in the sidebar; null is all of them. */
  category = $state<string | null>(null);
  filter = $state("");
  selected = $state<SecretName | null>(null);
  editing = $state<SecretDraft | null>(null);
  connections = $state<ConnectionRow[]>([]);
  /** The connection picked in the sidebar, by its saved name. */
  connection = $state<string | null>(null);
  /** The picked connection as it is being changed; null when nothing is picked. */
  draft = $state<ConnectionRow | null>(null);
  /** What the ribbon opened over the view: the categories, the password or the destroying of the vault. */
  dialog = $state<null | "categories" | "password" | "destroy">(null);
  error = $state<string | null>(null);
  busy = $state(false);
  copied = $state<string | null>(null);

  shown = $derived(shownSecrets(this.rows, this.category, this.filter));
  unlocked = $derived(this.info?.unlocked ?? false);
  /** Whether the connection being changed differs from the saved one. */
  changed = $derived.by(() => {
    if (!this.draft) return false;
    const saved = this.connections.find((c) => c.name === this.connection);
    return JSON.stringify(saved) !== JSON.stringify(this.draft);
  });
  problems = $derived(this.draft ? connectionProblems(this.draft) : []);

  /** Runs a command; its error shows instead of being thrown. A locked
   *  vault asks for its password first and the command goes on. */
  async #run<T>(command: string, args: Record<string, unknown> = {}): Promise<T | undefined> {
    this.error = null;
    try {
      return await unlock.invoke<T>(command, args);
    } catch (err) {
      this.error = String(err);
      // The password was not given: show the unlock form again.
      if (this.error === LOCKED && command !== "vault_info") {
        this.dialog = null;
        this.editing = null;
        this.info = (await this.#run<VaultInfo>("vault_info")) ?? this.info;
        this.rows = [];
        this.error = null;
      }
      return undefined;
    }
  }

  async refresh() {
    this.info = (await this.#run<VaultInfo>("vault_info")) ?? this.info;
    this.connections = (await this.#run<ConnectionRow[]>("connections_list")) ?? this.connections;
    if (this.info?.unlocked) await this.#loadSecrets();
    else this.rows = [];
  }

  async #loadSecrets() {
    this.rows = (await this.#run<SecretRow[]>("vault_secrets")) ?? [];
    this.categories = (await this.#run<string[]>("vault_categories")) ?? [];
    if (this.category && !this.categories.some((c) => c.toLowerCase() === this.category?.toLowerCase())) this.category = null;
  }

  async #slow(command: string, args: Record<string, unknown>) {
    this.busy = true;
    const info = await this.#run<VaultInfo>(command, args);
    this.busy = false;
    if (info) {
      this.info = info;
      await this.refresh();
    }
    return info !== undefined;
  }

  create(password: string) {
    return this.#slow("vault_create", { password });
  }

  unlock(password: string) {
    return this.#slow("vault_unlock", { password });
  }

  async lock() {
    this.info = (await this.#run<VaultInfo>("vault_lock")) ?? this.info;
    this.rows = [];
    this.editing = null;
  }

  async changePassword(current: string, next: string) {
    if (await this.#slow("vault_change_password", { current, new: next })) this.dialog = null;
  }

  async destroy(password: string) {
    if (await this.#slow("vault_destroy", { password })) {
      this.dialog = null;
      this.selected = null;
    }
  }

  startSecret() {
    // A vault with one category starts there; with a category picked, in that one.
    const only = this.categories.length === 1 ? this.categories[0] : "";
    this.editing = { was: null, category: this.category ?? only, key: "", value: "", description: "" };
  }

  async editSecret() {
    const row = this.rows.find((r) => this.selected && r.category === this.selected.category && r.key === this.selected.key);
    if (!row) return;
    const value = await this.#run<string>("vault_value", { name: { category: row.category, key: row.key } });
    if (value === undefined) return;
    this.editing = { was: { category: row.category, key: row.key }, category: row.category, key: row.key, value, description: row.description };
  }

  async saveSecret() {
    if (!this.editing) return;
    const rows = await this.#run<SecretRow[]>("vault_save_secret", { edit: this.editing });
    if (!rows) return;
    this.selected = { category: this.editing.category.trim(), key: this.editing.key.trim() };
    this.editing = null;
    this.rows = rows;
    this.categories = (await this.#run<string[]>("vault_categories")) ?? this.categories;
  }

  async deleteSecret() {
    if (!this.selected) return;
    const rows = await this.#run<SecretRow[]>("vault_delete_secret", { name: this.selected });
    if (rows) {
      this.rows = rows;
      this.selected = null;
    }
  }

  /** Puts the picked secret's value on the clipboard. */
  async copyValue() {
    if (!this.selected) return;
    const value = await this.#run<string>("vault_value", { name: this.selected });
    if (value === undefined) return;
    await navigator.clipboard.writeText(value);
    this.copied = `${this.selected.category} / ${this.selected.key}`;
  }

  /** The categories editor, over the categories and how many secrets each has. */
  categoryEditor(): CategoryEditor {
    const usage = new Map<string, number>();
    for (const r of this.rows) usage.set(r.category.toLowerCase(), (usage.get(r.category.toLowerCase()) ?? 0) + 1);
    return new CategoryEditor(this.categories, usage);
  }

  /** Keeps the list from the categories editor; renames move their secrets. */
  async saveCategories(names: string[], renames: CategoryRename[]) {
    const categories = await this.#run<string[]>("vault_save_categories", { names, renames });
    if (categories) {
      this.dialog = null;
      await this.#loadSecrets();
    }
  }

  /** The keys stored in `category`, for a field that points into the vault. */
  keysOf(category: string): string[] {
    return this.rows.filter((r) => r.category.toLowerCase() === category.trim().toLowerCase()).map((r) => r.key);
  }

  pickConnection(name: string | null) {
    this.connection = name;
    const saved = this.connections.find((c) => c.name === name);
    this.draft = saved ? structuredClone($state.snapshot(saved)) : null;
  }

  newConnection() {
    const name = freshName(this.connections.map((c) => c.name));
    this.connection = null;
    this.draft = { name, fields: [] };
  }

  addField() {
    this.draft?.fields.push(["", { mode: "plain", value: "" }]);
  }

  removeField(index: number) {
    this.draft?.fields.splice(index, 1);
  }

  /** Turns a field into plain text or a pointer into the vault, keeping what fits. */
  setMode(index: number, mode: FieldValue["mode"]) {
    const field = this.draft?.fields[index];
    if (!field || field[1].mode === mode) return;
    field[1] = mode === "plain" ? { mode: "plain", value: "" } : { mode: "vault", category: this.draft?.name ?? "", key: field[0] };
  }

  async saveConnection() {
    if (!this.draft || this.problems.length > 0) return;
    const rows = await this.#run<ConnectionRow[]>("connection_save", { was: this.connection, connection: this.draft });
    if (!rows) return;
    this.connections = rows;
    this.pickConnection(rows.find((c) => c.name.toLowerCase() === this.draft?.name.trim().toLowerCase())?.name ?? null);
  }

  async removeConnection() {
    if (this.connection === null) {
      this.draft = null;
      return;
    }
    const rows = await this.#run<ConnectionRow[]>("connection_remove", { name: this.connection });
    if (rows) {
      this.connections = rows;
      this.pickConnection(null);
    }
  }
}

export const vault = new VaultStore();
// The question can open the vault from any tab; the Vault tab then shows it open.
unlock.onUnlocked(() => vault.refresh());
