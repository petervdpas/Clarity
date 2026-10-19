# The vault

Clarity's own encrypted store of secrets and its connections: what it keeps, how it locks and unlocks, the Vault tab, and where it lives on disk.

## What it keeps

The vault keeps the secrets Clarity uses while you work, such as the API key of the [agent's connection](ai.connections.md). A secret has a category, a key, a value and a description. The description is not secret: it shows in the list, to tell secrets apart. Clarity notes when each secret was made and last changed, in UTC.

Beside the vault are the connections: named sets of fields, each plain text or a pointer to a secret by category and key. They let something ask for "the `apikey` of connection `anthropic`" without the key sitting in a file. Today the Agent panel is the part of Clarity that reads them.

## Locking

The vault is encrypted behind one master password. The key made from it is kept only in memory while the vault is unlocked, and forgotten when it locks. The vault locks itself after 15 minutes without use, and **Lock** in the ribbon locks it at once.

When something needs a secret while the vault is locked, such as sending a request to the agent, a dialog asks for the master password over whatever tab you are on. Unlock it and what you were doing goes on; **Cancel** stops it instead.

Write the master password down somewhere safe. Without it, nobody can open the vault, Clarity included.

## The Vault tab

The first time you open the Vault tab, it offers to make the vault: type the master password twice and click **Make the vault**. After that, the tab shows the vault locked or unlocked.

The ribbon switches between **Secrets** and **Connections**.

- **Secrets** lists every secret by category, key, description and last change, with a filter box. The sidebar lists the categories with how many secrets each holds; click one to see only those.
- **Add**, **Edit** and **Delete** work on the secret you select; double-click a row to edit it. **Show** in the form reveals a value while you type it.
- **Copy value** copies the selected secret's value to the clipboard.
- **Categories** adds, renames and removes categories. A secret needs a category, so add one first. Renaming a category moves its secrets; a category that secrets still use cannot be removed.
- **Connections** lists the connections in the sidebar. **New connection** adds one, and **Delete** removes the one you picked. You can edit connections while the vault is locked; unlock it to pick secrets from a list.
- **Change password** asks for the current master password and a new one.
- **Destroy vault** removes every secret for good, after you give the master password. The connections stay, but their vault fields then find nothing.

## Where it lives

Clarity keeps its vault in the `vault` folder and its connections in `connections.json`, both in `~/.clarity` in your home folder. Set the environment variable `CLARITY_HOME` to a folder to keep them there instead.

The vault folder holds one encrypted file per secret, named so that the folder says nothing about what it holds. `connections.json` is plain JSON: put anything secret in the vault, not in a plain field.

## Your applications

The vault and the connections belong to Clarity, never to a project. They are not in the project's folder, and an application Clarity generates never gets Clarity's vault. A generated application takes its database password from an environment variable instead; see [Data sources](data.sources.md).
