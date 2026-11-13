# Settings

The app's `settings` block: what each person using the app sets for themselves (Clarity's own language, theme and agent settings, and the app's own), secret, folder and reference settings, and the Settings editor.

## What settings are

What each person sets for themselves, such as the language, the theme, whether the toolbar shows or how many rows a page holds, are settings. A project has one `settings` block, in any module. Each person's settings are kept on their own machine, never in the database, and every window has them as its data `settings`.

## The Settings editor

Click **Settings** in the project tree (or **+ Settings** while the project has none). The view has two parts:

- **Clarity's own**: a checkbox each for **Language** (`language`), **Theme** (`theme`), **Agents may read while the app is closed** (`agentReadsWhileClosed`) and **Agents may change while the app is closed** (`agentChangesWhileClosed`). Ticking one adds its line; unticking takes it away.
- **The app's own**: a row per setting with its **Setting** name, **Type**, **Label**, **Default**, **Format** (for a type that has formats) and **Help**; the arrow before a row shows **Min** and **Max**. The type list offers only what a setting can be. The arrows move a setting, the cross removes it, and **Add setting** adds one.

Each person keeps a setting under its name, so a new name in a row changes only its line, and is not carried into windows and code. While the project has no settings, the first one starts the block in the first module.

Every change is written to the module at once and checked.

## A settings window

**New window** offers **Settings window** when the project has settings: a dialog with a labeled form of every setting and a Close button. Open it from a menu item or a button, as any window.

### `settings`

```text
settings
    language
    theme
    agentReadsWhileClosed
    agentChangesWhileClosed
    name: type, modifiers
```

```clarity
settings
    language
    theme
    showToolbar: bool, default true, label @showToolbarSetting
    exportFolder: text, label @exportFolderSetting
    pageSize: integer, min 10, max 200, default 50, label @pageSizeSetting
```

- A setting is written as a field of an entity: a camelCase name, a type and the modifiers `default`, `min`, `max`, `label`, `help` and `format`. Every setting has a `label` by key, as every field has.
- Its type is a plain one: `text`, `integer`, `decimal(p, s)`, `money`, `bool`, `date`, `datetime`, `duration`, `email`, `url`, `color`, an enum, `secret` (see [Secret settings](#secret-settings)), `folder` (see [Folder settings](#folder-settings)) or a reference to a record (`openProject: ref Project`).
- A setting not kept yet has its `default`; a kept value that no longer fits (a setting removed, an enum value gone, a number past its ends) is dropped when the app starts.

The checker refuses:

- a second block: "a project has one 'settings' block";
- a list, a picture, a file or an id: "a setting is a plain value or a reference; 'tags' cannot be list<text>";
- `hashed`: "only a stored entity's field is hashed; a record or a setting keeps its value as it is";
- an entity named `Settings`: "'Settings' is the name of the project's settings; give this entity another name", and window data called `settings`: "every window has 'settings' of its own; give this data another name".

### Clarity's own settings

`language`, `theme`, `agentReadsWhileClosed` and `agentChangesWhileClosed` are written as a bare word, without a type or modifiers.

- `language` and `theme` are the language and the theme shown, labeled with Clarity's words `languagePicker` and `themePicker`. Setting either switches the language or the theme, as the pickers of the click targets `language` and `theme` do, and those pickers change the same settings. A dropdown bound to `settings.language` offers the project's languages by their own names, and one bound to `settings.theme` the offered themes by their titles.
- `agentReadsWhileClosed` and `agentChangesWhileClosed` are bools, labeled with Clarity's words `agentReadsWhileClosedSetting` and `agentChangesWhileClosedSetting`, that allow AI agents to read and to change through the app's MCP service while the app is closed. They start as the project file's `agents while closed` line says (see [The project file](project.file.md)), off without it. They belong to an app with a command for agents; without one the checker says "agentReadsWhileClosed is a setting of an app whose commands agents call; no command says agent". See [Agents](project.agents.md).

## Kept per person

Each person's settings are kept as `settings.json` in a folder named after the application, in the folder for application settings of their system: `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` or `~/.config` elsewhere. In a browser, the web program keeps them in the person's browser. A secret is the exception: it is kept in the system's protected store.

## In every window

Every window, every view and the app have `settings` as their data, without a `Data` element. A part binds `settings.pageSize`, a form binds `settings`, and code reads and sets `settings.pageSize`. A change takes effect at once and is kept, and every window sees it.

```xml
<Dialog class="shop.Preferences" title="@preferencesTitle">
  <Checkbox name="showToolbar" at="1,1" span="6" caption="@showToolbarSetting" bind="settings.showToolbar" />
  <Number name="pageSize" at="1,2" span="3" bind="settings.pageSize" />
</Dialog>
```

```clarity
code Orders
    handler nextPage
        offset = offset + settings.pageSize
```

## Reference settings

A `ref` setting keeps the record's id, and is empty until the person picks one.

```clarity
settings
    openProject: ref Project, label @openProjectSetting
```

- Code reads it as a `ref` field: `settings.openProject != none`, `let open = settings.openProject`, `settings.openProject = project`.
- A dropdown bound to it offers the entity's records.
- A kept id that is not a whole number above 0 is dropped. When its record is deleted, loading it fails as for any reference, so code that deletes the record clears the setting.

## Secret settings

A token, a key or a database's connection is a setting of type `secret`:

```clarity
settings
    apiToken: secret, label @apiTokenSetting
```

- A secret is kept in the system's protected store, per application and per person: the Credential Manager on Windows, the Keychain on macOS, the Secret Service (such as GNOME Keyring or KWallet) elsewhere. It is never in `settings.json`.
- Only a password input shows it. A text box or a text part bound to it is an error; a settings window gives it a password input, whose buttons are Clarity's words `showSecret`, `hideSecret` and `copySecret`.
- In code it is text: `settings.apiToken = value` keeps it, and `settings.apiToken = ""` forgets it.
- It has no default ("a secret has no default: it is the person's own ('apiToken')"), and only a setting is a secret: an entity's field that is one is an error.
- The project file names it as `"setting:apiToken"`: a web API signs its calls with it (`auth bearer "setting:apiToken"`), a server database connects with it (`datasource main postgres "setting:shopDb"`), and a sign-in provider keeps its app's secret in it (`secret "setting:githubSecret"`). A name there that is no secret setting is an error that says to add it as `name: secret`. While a database's secret is not kept, the database does not open; see [Data sources](data.sources.md#passwords-stay-out-of-the-project).

## Folder settings

The folder a file store keeps its files in can be each person's own choice, chosen while the application runs: a setting of type `folder`.

```clarity
settings
    photosFolder: folder, default "photos", label @photosFolderSetting
```

The project file points the store at it:

```clarity
filestore photos "setting:photosFolder"
```

And a folder part in a dialog shows it, in `design/Preferences.cxaml`:

```xml
<Dialog class="records.Preferences" title="@preferencesTitle">
  <Folder name="photosBox" at="1,1" span="6" bind="settings.photosFolder" />
</Dialog>
```

- It is kept with the settings, in `settings.json`, like any other setting. Until the person chooses one, the store is in its `default`; without a default, in a folder named after the store.
- A relative folder is in the folder the settings file is in (such as `~/.config/records/photos`); an absolute one is used as it is.
- It is read at every use: a folder chosen while the application runs is where the next file goes, at once. Files already in the old folder stay there: moving them is the person's choice.
- The folder part shows it: a text field with the path and a button, in Clarity's word `chooseFolder`, that opens the system's folder picker. In a browser there is no picker, so the path is typed. Only a folder part shows a folder setting.
- In code it is text: `settings.photosFolder = "/mnt/photos"`.
- Only a setting is a folder (an entity's field that is one is an error), and only a folder setting can be a store's `setting:name`: anything else is an error, "file store photos keeps its files in setting 'photosFolder', which is no folder setting; add it to settings as photosFolder: folder".

See also [Pictures and files](data.pictures-files.md), [Data sources](data.sources.md), [Languages](languages.md) and [Themes](themes.md).
