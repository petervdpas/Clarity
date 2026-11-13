# Vault

Status: built. The IDE's own encrypted store of secrets and its connections (agent API keys, refresh tokens); generated apps do not use it.

Progress: 90%

## Code
- Core: `crates/clarity-vault/src/` (vault.rs: `vault.json` with id, Argon2id work factors and salt, and a canary that proves the master password; `DEFAULT_PARAMS` 64 MiB, 3 iterations, parallelism 4 (:46); secrets.rs: one AES-256-GCM record per secret in `secrets/`, holding category, key and value, bound to the vault and its slot; connections.rs: named fields, plain or a pointer to a secret by category and key, in `connections.json`)
- IDE backend: `app/src-tauri/src/vault.rs` (folder `~/.clarity` or CLARITY_HOME (:22-26); create, unlock, lock, secrets, categories with renames, change password, destroy, connections, `resolve`; locks itself after 15 minutes unused, `AUTO_LOCK` :20)
- IDE: `app/src/lib/VaultView.svelte` (536 lines), `VaultSide.svelte`, `vaultStore.svelte.ts`, `vault.ts` (CategoryEditor), `UnlockDialog.svelte`, `unlock.ts`, `unlockStore.svelte.ts`, `SecretField.svelte` (eye and copy)
- Keys: layers vault, vault.secrets, vault.connections; Ctrl+4 opens it, Ctrl+L locks; Change password and Destroy vault have no key (`runtimes/web/src/host/keys.ts:18-50`)
- Readers: Clarity's agent connections ([agent](agent.md)); Foundry refresh tokens in category `Microsoft`, the connection's name as key (`app/src-tauri/src/foundry.rs:17-49`)
- Docs: manual/vault.md, manual/ai.connections.md

## Tests
- `crates/clarity-vault/tests/vault.rs`, connections.rs
- `app/src/lib/vault.test.ts`, unlock.test.ts

## Open
- Vault keys, the eye and copy on fields inside an open vault, AltGr on European layouts: UNTRIED on Windows (WINDOWS-TESTS.md:23-28, "no vault was made"); see [os-checks](os-checks.md)
- A designed app's own vault or config parts: NOT BUILT; apps keep `secret` settings in the system keychain, see [settings-secrets](settings-secrets.md)
- The vault's crypto as a shared crate for encrypted files and stores: NOT BUILT (APP-CAPABILITIES-PLAN.md:362, 384); see [encrypted-storage](encrypted-storage.md)

## Conflicts
- none

## Decisions
- 2026-10-19: Clarity holds its own vault and connections, in Clarity's own format (Argon2id + AES-256-GCM), with no compatibility with other tools; a designed app gets its own only when its design adds config parts, never Clarity's (memory: clarity-status.md)
- 2026-10-25: secret fields get an eye to show and a button to copy. User: "passwords, secrets and alike should have an option (open eye/closed eye) to show it or copy it"
- What cannot be undone gets no key (Change password, Destroy vault) (DESIGN 5.28)
- When pointed at another project as a model, port its behaviour and never name that project in Clarity (memory: reference-the-named-source.md)

## Was
- memory: clarity-status.md (the vault paragraph, 2026-10-19), reference-the-named-source.md
