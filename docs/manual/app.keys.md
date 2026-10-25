# Keyboard shortcuts

The keys that Clarity itself responds to, by where you are in the window.

On macOS, Cmd works wherever this page says Ctrl.

You can change Clarity's keys: **Clarity settings**, **Keys**. Pick **Change** beside a command and press the new key. A key another command has in the same place offers to swap the two; **Reset** puts a key back, **Reset all** every one. Any command there can get a key, also one that has none. Keys that typing and dialogs need (letters without Ctrl or Alt, Tab, Enter, Escape) cannot be used. The tables below give Clarity's own keys; a ribbon button's tooltip shows the key it has now.

A key works in the view that is open (the ribbon's tab), and the keys under Anywhere work in every view. While the cursor is in a text field, the field keeps its own keys, such as Delete, Ctrl+Z and Ctrl+C. Nothing runs behind an open dialog.

## Anywhere

| Key | Does |
|---|---|
| F1 | Open the manual on the page about the tab you are on (the agent's page while you type in the Agent panel) |
| Ctrl+1 to Ctrl+4 | The Project, Design, Preview and Vault views |
| Ctrl+O | Open a project |
| Ctrl+Shift+N | New project |
| Ctrl+, | Clarity settings |

The Agent panel has no key of its own; give it one in Keys if you like.

## The manual

| Key | Does |
|---|---|
| Alt+Left | Back to the page you came from |

## Project view

| Key | Does |
|---|---|
| F5 | Reload the project |
| F7 | Check the whole project |
| Ctrl+Shift+W | New window (also in the Design view) |

## Design view

| Key | Does |
|---|---|
| Ctrl+Z | Undo the last change |
| Ctrl+Y or Ctrl+Shift+Z | Redo |
| Ctrl+G | Show or hide the grid |
| Delete or Backspace | Remove the picked part, region or command |
| Escape | Pick nothing; while dragging, cancel the drag |

Undo also takes back changes made on other tabs, such as a settings change or a turn of the agent.

Delete, Backspace and Escape do nothing to the window while the cursor is in a text field, such as a property. To remove the whole window, use **Delete** in the ribbon with nothing picked; Clarity asks first.

## Preview view

| Key | Does |
|---|---|
| F5 | Run: generate, build and start the application |
| Ctrl+Shift+B | Build a release program |
| Shift+F5 | Stop the running application |
| Alt+Left | Back to the window before |
| Alt+Home | The main window |

## Vault view

| Key | Does |
|---|---|
| Ctrl+L | Lock the vault |
| Ctrl+N | Add a secret, or a new connection in Connections |
| F2 | Edit the picked secret |
| Delete | Delete the picked secret, or the connection in Connections |
| Ctrl+Shift+C | Copy the picked secret's value |

Change password and Destroy vault have no key: what cannot be undone is not one key away.

## Text panel and other text editors

These keys work in the text panel of the designer (Design and Code), and in the text editors of modules, functions and themes.

| Key | Does |
|---|---|
| Ctrl+S | Save the text |
| Escape | Close the panel |
| Tab | Put in four spaces |
| Enter | New line with the same indentation, one level more after a line that opens a block |

See [Design as text](design.text.md).

## Agent panel

| Key | Does |
|---|---|
| Enter | Send the message |
| Shift+Enter | New line in the message |

See [The agent](ai.agent.md).

## Dialogs and names

| Key | Does |
|---|---|
| Escape | Cancel the New project, New window, New entity and New enum dialogs, and the vault password dialog |
| Escape | In the name field of an entity or enum editor: put the old name back |

## In your application

The keys of the application you design are part of its design, not of Clarity: a button's `key`, Enter for the `default` button, Escape for the `cancel` button, and the keys of menu items. See [The app and its frame](design.frame.md) and [Parts](design.parts.md).
