# The Clarity language

This is the reference for the Clarity language: every construct the checker accepts today, with its syntax and its rules. It is written for people and for AI assistants alike. When this file and the parser disagree, the parser is right and this file has a bug.

A Clarity project is written entirely in Clarity: the project settings, the data and the behavior in `.clarity` files, and each window and view in a `.cxaml` file of its own. Clarity transpiles it to a real application in the stack the project names. Why the language looks the way it does is in [DESIGN.md](DESIGN.md); this file only says what it is.

Contents:

1. [Lines, names and values](#1-lines-names-and-values)
2. [The project file](#2-the-project-file)
3. [Modules](#3-modules)
4. [Data: enums and entities](#4-data-enums-and-entities)
5. [Windows](#5-windows)
6. [Parts](#6-parts)
7. [The app and its frame](#7-the-app-and-its-frame)
8. [Code behind](#8-code-behind)
9. [Functions and actions](#9-functions-and-actions)
10. [Statements and expressions](#10-statements-and-expressions)
11. [A complete example](#11-a-complete-example)
12. [Grammar summary](#12-grammar-summary)

## 1. Lines, names and values

Clarity is line based. Each declaration, setting, part or statement is one line, and a block is the lines indented below it.

- **Indentation** uses spaces; a tab is an error. Four spaces per level is the convention, and the designer writes four.
- **Comments** start with `//` and run to the end of the line. They can stand on their own line or follow code. The designer keeps them when it rewrites a file.
- **Blank lines** are free and are kept too.
- **Names** start with a letter, followed by letters and digits: `customer`, `vatNumber`, `OrderLine`. Names are case sensitive. Entities, enums and their values, windows and the app are written in `PascalCase`. Every other name must be `camelCase`, letters and digits only, starting with a small letter: the parts of a window and their handlers (`newGame`, `square1`, `saveClicked`), and fields, settings, data, functions, actions, parameters, locals, constants and what an engine keeps (`vatNumber`, `withinCredit`); the checker reports any other. Two kinds of name are left as written: the functions an engine answers its host by (`clarity_tick`, `clarity_draw`, `clarity_pointer`, `clarity_key`), and a name with one `_` after it, which steps around a word Clarity keeps (`key_`). A field is stored in the column of its name in snake case (`vatNumber` in `vat_number`) unless it names its `column`; every stack turns names into its own style.
- **Every part has a name** (`button @saveCaption name save at 1,3`), which its events and code go by. The designer names a part after what it shows or is for when it is placed (`ordersTable`, `email`), or `button1` when it does not know yet, and renames a numbered part after its purpose once it knows it (a label given `for vatNumber` becomes `vatNumberLabel`), unless code goes by the number. A part written without a name is an error, and **Name them** under Problems names every such part in the file the same way.
- **Keys** name the texts people read: `@saveCaption`, `@customerCardTitle`. A key's text is in the language files, one per language (see [Translations](#translations)); a title, a caption, a label, a message are keys. Only a text without letters, such as a calculator's `"7"`, is written as it is.
- **Strings** are written in double quotes: `"logo.png"`. Inside them, `\"` is a quote, `\\` a backslash and `\n` a new line. Other backslashes are kept as written, so `"^[0-9]\d+$"` is a valid pattern.
  In code, a string is also read for slots: `{customer.name}` is a value, and `{{` and `}}` are braces (see [Text with slots](#text-with-slots) and [Texts by key](#texts-by-key)). Strings in settings, such as patterns and file names, have no slots.
- **Numbers** are written without quotes: `12`, `9.95`.
- **Booleans** are `true` and `false`.

Words such as `page`, `data` and `bind` are keywords only where the grammar expects them. Outside those places they are ordinary names, so a field may be called `data` and a part may be called `close_button`. The words `and`, `or`, `not` and `in` are reserved in expressions.

A text people see is a key (`label @customerVatNumberLabel`), whose text the language files give; see [Translations](#translations). **Make texts keys** writes a key for each text a project wrote as it was, and gives it a text derived from the name where there was none: `vatNumber` becomes "Vat number", and `CustomerCard` becomes "Customer card".

## 2. The project file

A project is one file, `<name>.clarity`. Next to it is a folder with the same name, which holds the modules, the generated output and the data:

```text
shop.clarity           the project file: open this one
shop/
    customers.clarity  modules
    sales.clarity
    migrations/        the steps that bring each database to the dictionary
        main/          one folder per SQL data source
            0001.clarity
    out/               the generated application
    data/
```

The project file holds one `project` block:

```clarity
project Shop
    title "Shop"
    icon "shop.png"
    stack rust desktop
    output "out"
    language "en"
    currency EUR
    datasource main sqlite "data/shop.db"
    datasource payments rest "https://api.example.com/v2"
        auth bearer "env:PAYMENTS_TOKEN"
```

| Setting | Required | Meaning |
|---|---|---|
| `title "text"` | no | The name people see, the program's name: its settings folder and package use it, and a window shows it when neither the window nor the app has a title. Plain text, without slots: a version shows in a window's or the app's title with `{app.version}`. When it is left out, it is derived from the project name. |
| `icon "app.png"` | no | The application's icon: a picture among the project's resources (PNG, JPEG, WebP or SVG, square, best 256 by 256 or larger). Its windows and taskbar button show it on every desktop stack, its web page's tab, and on Windows its program file: Clarity draws every size Windows asks for and writes them into the generated program's `windows/` folder (`app.ico`, which C# names as its `ApplicationIcon`, and a resource object per processor, which Rust, C and C++ link, and Go as `resources_windows_<processor>.syso` beside `main.go`, together with the version when the project has one), so no build needs a tool of its own. Without it, Clarity's own. A resources folder that holds `icon.png` while the project names no icon gets a warning to name it. |
| `version "1.2.0"` | no | The application's version, as `major.minor.patch` with an optional pre-release (`2.0.0-beta.1`). Code reads it as `app.version`, and a text by key shows it with the slot `{app.version}`, which Clarity fills itself (`welcomeVersion "Version {app.version}"`), so a splash, a title or an About dialog shows it where the designer puts it. Each stack's package carries it (C#'s `<Version>`, Rust's `Cargo.toml`, CMake's `project(... VERSION)`), and on Windows the program file's Properties show it. Optional: without it nothing shows a version, and a use of `app.version` is an error that says to add the line. |
| `stack language ui target...` | yes | The stack the project is transpiled into, read by position. Languages: `rust`, `go`, `csharp`, `c`, `cpp`. No language names a UI (the windows are drawn by Clarity.UI), so the targets follow the language: `stack csharp desktop`, `stack go desktop`. Targets: `desktop` for the program with windows, `web` for a web server that serves it to a browser (every language; the web program of Rust, Go, C and C++ serves the shared screen and is generated alone, `stack go web`, until Clarity.UI draws in the browser; with both, `--serve` serves in place of the window; `--address host:port` says where), and `cli` for a command-line program beside it (`stack go desktop cli`); a project whose only target is `cli` names no UI (`stack go cli`). See [Commands](#commands). |
| `output "folder"` | no | Where the generated application goes, relative to the project folder. The default is `out`. |
| `output keeps "name", ...` | no | Folders in the output that are the project's own, which Clarity leaves alone: each the name of one folder, not a path. |
| `language "tag"` | no | The language the project's texts are written in, as a language tag: `"en"`, `"nl-NL"`. Dates and numbers follow the locale of the person using the application; this language where the system names none. |
| `currency CODE` | no | The currency of `money` fields, as its ISO 4217 code: `EUR`, `USD`. Without it, amounts show without a currency sign. |
| `keys editable` | no | The people using the application set its shortcut keys themselves: a menu item or button with `on click -> keys` opens Clarity's keys window, and each person's keys are kept in their settings file. Without it, the keys are the design's, and `-> keys` is an error. |
| `agents while closed read change` | no | What AI agents may do through the app's MCP service while the app is closed and nobody is there to allow it: `read`, `change` or both. It is where the settings `agentReadsWhileClosed` and `agentChangesWhileClosed` start, until the person using the app sets them. Without the line, agents may do nothing while the app is closed. |
| `agents at most 3` | no | How many AI agents the running app serves at once through its MCP service: a whole number from 1 to 100, 3 without the line. One more reads why it is refused, and the app's window says it was turned away. |
| `port 8080` | no | Where a web application (the `web` target) listens, on this computer: a number from 1 to 65535, 8080 without the line. Started with `--address host:port` or `CLARITY_ADDRESS` it listens there instead. |
| `row height 30` | no | How high a row of the windows' grid is, in pixels: a whole number from 24 to 64, 36 without the line. Inputs and buttons are a row high, and a table's rows are never higher; the designer, the preview and every stack lay out by it. Clarity's own fields are 30 high. |
| `engines interpreted` | no | How the engines written in Clarity run: `engines compiled` (the default, without the line) turns their code into WebAssembly; `engines interpreted` carries their code in the file and runs it with Clarity's interpreter. Both give the same answers on every stack. |
| `theme Name...` | no | The themes the application offers, declared in modules, the first being the one it starts with: `theme Shop Night`. People switch with the click target `theme`. Without any, the application looks as Clarity does, and parts cannot have a `style`. See [Themes](#themes). |
| `datasource name kind "where"` | no | A data source. Kinds: `sqlite`, `postgres`, `mysql`, `mssql`, `rest`, and `csv`, `json`, `xml` or `yaml` for a file in that format (always `external`, its format's options on indented lines, such as `separator ";"`; its records are read when the program starts and the file is written again after each change). "Where" is a file path for SQLite (relative to the project's folder, for Clarity and for the generated application), a connection string or URL for the other databases (see Databases in section 4), and the base URL of a web API. `external` after it marks a database Clarity does not own, and `readonly` one whose records are only read. |
| `signin Entity on desktop web cli` | no | Who is using the program: the entity of people, and the targets people sign in on (`web` alone without `on`). Indented under it, the strategies the project uses and their options; see [Signing in](#signing-in). |
| `filestore name "folder"` | no | A file store: the folder that keeps the pictures of `image` fields with `store name`. A relative folder is in the project's folder, as a SQLite file is; `"env:NAME"` takes it from that environment variable, for a folder that differs per machine; `"setting:name"` takes it from a folder setting each person chooses while the application runs (see [Folder settings](#folder-settings)). |
| `auth kind "secret"` | no | Indented under a `datasource` line, for web APIs. Kinds: `bearer`, `api-key`, `basic`. The secret names where to find it, such as `"env:NAME"` or `"setting:name"`, a secret setting the system's keychain keeps; it is never written into the file itself. A generated application reads it from `env:NAME` or from the secret setting `name` (see [Secret settings](#secret-settings)). |
| `paging kind ...` | no | Indented under a web API's `datasource` line: how it hands out a long list, for `every page`. `paging page "page" size "per_page" 100` sends page numbers from 1, `paging offset "offset" size "limit" 100` offsets from 0, `paging cursor "after" next "meta.next"` the cursor each page names at that path, and `paging link` follows the `Link` header's next address. `items "data"` at the end says where a page's items are in its JSON. |
| `openapi "file"` | no | Indented under a web API's `datasource` line: its OpenAPI (Swagger) description, a YAML or JSON file of the `apis` folder. Clarity writes the module `apis/<source>.clarity` from it, for what the project picks, and writes it again when the file or the picks change. |
| `pick "operationId", ...` | no | Indented under a web API with an `openapi` line: the operations the application uses. Each becomes an action, and the schemas it takes or gives entities. `pick schema "Name", ...` makes schemas entities on their own. A pick the description does not have is a warning. |

Each setting appears once, except `datasource` and `filestore`. An unknown setting or value is an error that names the line and lists the accepted words.

### Signing in

A program used by more than one person knows who is using it. The project file names the entity of people and how they prove who they are, each way a strategy of Clarity's catalog, several at once:

```text
project Club
    stack go desktop web cli
    signin Member on desktop web cli
        password email passwordHash
            minimum 12
        recovery email recoveryCodes
        account login
        token apiToken
        session 8 hours
        tries 5 per 15 minutes
        remember 30 days
```

| Line | Meaning |
|---|---|
| `password field hashField` | A name and a password: the person whose `field` equals the name, with the password checked against the `hashed` field (Argon2id). The sign-in view's fields for `name` and `password` give them. |
| `minimum 12` | Indented under a strategy whose secret people choose: text assigned to its hashed field has at least that many characters, else the save is refused with Clarity's `secretTooShort`. |
| `recovery field codesField` | A recovery code, once each: the codes are kept in the hashed field as their digests, and a sign-in uses one up. The parts `name` and `code` give them. |
| `account field` | The computer's own account: the person whose `field` equals the system user name. Desktop and command line. |
| `token field` | A token, kept hashed (SHA-256). The part `token` gives it. |
| `github field` | GitHub, in the system's browser: the person whose `field` equals their GitHub login (a text field) or their verified primary e-mail address (an e-mail field). Indented under it: `client "..."` (the OAuth app's id), `secret "setting:name"` (the secret setting holding the app's secret; GitHub needs it), `register`. Desktop. |
| `microsoft field` | A Microsoft account (work, school or personal), in the system's browser: matched by its e-mail address or login. Indented: `client "..."`, `tenant "..."` (the company, `organizations` or `consumers`; `common` without it), `secret "setting:name"`, `register`. Desktop. |
| `openid field` | Any OpenID provider, in the system's browser: matched by its e-mail address or login. Indented: `issuer "https://..."` (its endpoints are found by discovery), `client "..."`, `secret "setting:name"`, `register`. Desktop. |
| `session N minutes\|hours\|days` | How long a sign-in lasts; 8 hours without the line. |
| `tries N per M minutes` | Failed tries per account and address before it slows down; 5 per 15 minutes without the line. |
| `remember N days` | On the desktop, the program keeps a sign-in that long in the system's key store. With `when asked` (`remember 30 days when asked`) only when the person ticks Keep me signed in (Clarity's `signinRemember`): the sign-in part's box, or a sign-in view's `signinRemember`. |
| `shows password, recovery, account` | The strategies the sign-in part offers, in that order, the first being the default; only strategies the block names. Without it, every strategy of the block that works on the target, in the block's order. |
| `language picker` | A dropdown of the program's languages, each by its own name, headed by Clarity's `languagePicker`; picking one shows every text in it. The sign-in part shows it above its fields, and a sign-in view may hold it (`signinLanguage`). Without the line, there is none. |

A provider's secret is never written in the project file, only the secret setting that holds it. With `register`, someone without a record who signs in at the provider gets one, made from the field the provider matches; without it only people already known sign in (and the first person, as for every strategy that can make the first account). On a sign-in view a `signinButton` of a provider shows Clarity's `signinGithub`, `signinMicrosoft` or `signinOpenid`.

Each field a strategy names is a field of the entity, of a type the strategy takes (text or email for a name); a secret is `hashed`. A strategy that can make the first account (password, account) makes it from what was typed when there is no record yet, the other fields at their defaults: a required field without a default is an error that names it. People who sign in on the desktop or the web need a sign-in window: a page of the kind `signin` (`<SignIn>`), at most one and only with `signin`, or the app's splash holding the sign-in part (or a hand-built sign-in), which then stays open until someone signs in. Both at once is an error that says to keep one.

The sign-in window is built as every window is: window, content, views. Its content shows the sign-in views, one at a time; the first it names opens first. It holds that content and decoration: a heading, a label, a picture, a line or a link. A sign-in view holds only the sign-in parts and that decoration, and each sign-in part stands on a sign-in view only:

| Part | What it is |
|---|---|
| `signinField` | A label and a field for one input of a strategy: `input name`, `password`, `code` or `token`, the inputs of the block's strategies; a password field for `password` |
| `signinButton` | Signs in with a strategy of the block (`strategy password`), with what was typed in the sign-in fields of its own view; one that takes nothing typed (`account`) signs in at once |
| `signinRemember` | Keep me signed in, with `remember ... when asked` in the block |
| `signinLanguage` | The language dropdown, with `language picker` in the block |
| `signinOther` | A link that shows another sign-in view (`shows CodeView`): another way to sign in |
| `signinRefusal` | Where a refused sign-in says why; without one, a refusal shows as a message |

Each shows Clarity's own words unless its caption says others, and keeps only its place and look. A view has each at most once (a field per input, a button per strategy), and a button needs a field on its view for each input of its strategy. Switching between ways of signing in is switching views:

```clarity
page SignIn: signin
    content name ways at 1,1 span 12 rows 10
        view PasswordView
        view CodeView

page PasswordView: view
    signinField name nameField at 1,1 span 12 rows 2 input name
    signinField name passwordField at 1,3 span 12 rows 2 input password
    signinButton name signInButton at 1,5 span 12 strategy password
    signinOther name useCode at 1,6 span 12 shows CodeView
    signinRefusal name refusal at 1,7 span 12

page CodeView: view
    signinField name codeName at 1,1 span 12 rows 2 input name
    signinField name code at 1,3 span 12 rows 2 input code
    signinButton name codeButton at 1,5 span 12 strategy recovery
    signinOther name back at 1,6 span 12 shows PasswordView
```

New window's sign-in window makes these from the block: a view per strategy that takes typed inputs, the others as buttons on the first view, and an other way between each two.

The sign-in part `signin` goes on the app's splash, at most one in a project and only with `signin`. It draws the sign-in from the project's block, in Clarity's own words: per strategy what it takes (password: a name, a password and a Sign in button; recovery: a choice to use a recovery code, then a name and the code; token: a token field; account: a button that uses the computer's account), and a refusal inside itself. It keeps only its place and look: the block's `shows` line names what it offers, and its `remember ... when asked` the box that keeps a person signed in. The splash may hold a sign-in built by hand instead: parts named after a strategy's inputs and buttons that click to `signin <strategy>`.

In the language:

- `hashed` on a text field of an entity: assigning text stores its hash (`member.passwordHash = typed`); code never reads it and no part shows it.
- `me` is the record of who signed in, in handlers, actions, data events and commands: `me.name`, `me.role`. It cannot be set, and a function, which takes values only, does not read it. Without `signin` it is an error that says to add the line.
- `allow when <condition>` on an action or a command, before its statements, and `allow="<condition>"` on a window, a view and a command of a menu, toolbar or ribbon: who may run or open it. The condition is bool and reads `me`; the program refuses what is not allowed with Clarity's `notAllowed`.
- `readable when <condition>` and `writable when <condition>` on lines of an entity, with its record named as in its data events: the program refuses a find or a save of a record that does not fit with Clarity's `notYours`. Rows are never filtered by these lines; code writes `where n.owner == me`.
- `newToken()` gives a new random token as text, and `newRecoveryCodes(10)` that many recovery codes as lines of text, to show the person once and assign to the hashed field.
- On the app's splash a button signs in with `on click -> signin password` (in a `.cxaml` file `opens="signin password"`), taking each input of the strategy from the part of that name; `on click -> signout` in a menu, toolbar or button signs out.

```clarity
entity Member
    name: text(80)
    email: email, unique, required
    passwordHash: text, hashed
    role: Role, default Plain
    readable when me.role == Admin or member.id == me.id
    writable when me.role == Admin
```

```clarity
action promote(member: Member)
    allow when me.role == Admin
    effects db.write
    member.role = Admin
    save member
```

Five stacks are generated today: C#, Rust and Go with Clarity.UI, and C and C++ with the shared screen (webview; SQLite only for now).

### Translations

Every text a person reads is a key, written with `@`: titles, captions, labels, help texts, placeholders, tooltips, tab titles, enum labels, menu items, the status text, messages, refusals, and the captions and errors code sets. A file per language, in the `translations` folder of the module folder, gives each key its text in that language. The project's `language` is the default language, which has a file too:

```clarity
// shop/translations/en.clarity: the default language
translation en
    customerCardTitle "Customer card"
    savedName "Saved {name}"
    yes "Yes"
    no "No"
```

```clarity
// shop/translations/nl.clarity
translation nl
    customerCardTitle "Klantkaart"
    savedName "{name} opgeslagen"
    yes "Ja"
    no "Nee"
```

- The file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`.
- Every language file has the same keys: a key one file has and another lacks is an error. A key is camelCase, as names are, and is used anywhere in the project.
- The default language gives every key a text; an empty one is an error. Another language may leave a key empty (`savedName ""`): it shows the default's text until it is translated.
- A key's text may have slots, which code gives values by name: `message @savedName(name = customer.name)`. Each language keeps the slots of the default's text and may move them; a slot missing or added is an error, and so is a use that leaves one out or gives one the text does not have.
- A key no design or code uses is a warning; it can go. A text with letters written in quotes where people read it is an error that asks for a key.
- `yes` and `no` are what the screen shows for bools; `noPicture`, `loadPicture` and `removePicture` what an image part bound to a picture shows of its own. `languagePicker` and `themePicker` head the pickers that the click targets `language` and `theme` open (`languagePicker` also heads the sign-in part's language dropdown); a project that uses them gives them their texts. `showSecret`, `hideSecret` and `copySecret` name the buttons of a password input; a project with one gives them their texts. `unconnectedNotKept` and `unconnectedFailed` say why the databases did not open (see [Secret settings](#secret-settings)); a project with a secret connection or an `on unconnected` handler gives them their texts, and `{problem}` in the second is the database's own message. Every project gives them their texts.
- The application shows the language for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else any other region of Dutch, else the default. The designer can show the canvas and Preview in any of the project's languages.
- A project written before texts had keys gets them with **Make texts keys**, or `clarity spec keys <folder>`: each text becomes a key named after where it is (`saveCaption`, `customerNameLabel`, `cardTitle`), its text the default language's value, and old translation files give the same keys their translations.
- A menu item or button with `on click -> language` lets the person choose another language while the application runs: it lists the project's languages, each by its own name (Nederlands, Deutsch), and the application remembers the choice for its next start.
- A key's value can point at a file of the resources instead of being a text: `gameRules resource "rules-nl.md"`. A markdown part (`markdown @gameRules`) or an image (`image @bannerPicture`) shows by its key the file each language points it at; see [Resources](#resources). Such a key points at a file in every language: a markdown's at a Markdown file, an image's at a picture. A part that shows text cannot use a key that points at a file. A new language starts each such key at the default language's file.

### Resources

The `resources` folder of the module folder holds the files the application carries: its fixed pictures, such as a logo (`.png`, `.jpg`, `.gif`, `.bmp`, `.webp` or `.svg`), its Markdown files, such as the rules of a game, and any other file it needs, such as a PDF manual or a drawing (`.dxf`). Each is a file with a name of its own; how they are named is up to the project (`rules-en.md`, `rules-nl.md`). A part shows one in two ways:

- By its `source`, the same file in every language: `image source "logo.png"`, `markdown source "license.md"`.
- By a key, a file per language: the part names the key (`markdown @gameRules`), and each language file points the key at the file that language shows (`gameRules resource "rules-nl.md"`).

Every generated application carries the folder: next to the renderer on the web stacks, as embedded resources on C#. The designer shows the resources as they are in the folder, and shows them again when the folder changes.

#### Pictures

An image part shows a picture by its file name, or the picture its key points each language at (`image @bannerPicture`, with `bannerPicture resource "banner-nl.png"` in the Dutch file):

```xml
<!-- records/design/AlbumBrowseView.cxaml, with records/resources/logo.png -->
<View class="records.AlbumBrowseView">
  <Image name="collectionLogo" at="12,1" span="1" rows="1" source="logo.png" />
</View>
```

- The picture must be in the `resources` folder; a name that is not there is an error that lists the pictures that are.
- A picture is a `.png`, `.jpg`, `.gif`, `.bmp` or `.webp` file, which every stack shows without extra libraries.
- The picture keeps its proportions inside the part.

#### Markdown

A markdown part shows a Markdown file of the resources, a file per language through a key:

```clarity
// noughts/resources/rules-en.md and rules-nl.md
sidebar left name help width 320 grid 1 columns
    markdown @gameRules name rulesText at 1,1 span 1 rows 12

// noughts/translations/nl.clarity
translation nl
    gameRules resource "rules-nl.md"
```

- `markdown source "license.md"` shows one file in every language; `markdown @gameRules` the file each language points the key at. The key's value is a Markdown file in every language, never a text: long texts belong in the resources, not in the language files.
- A markdown part can show a text instead, `bind notes`, of a field or a plain value of the window. It shows one of the three: a key, a source or what it binds.
- Every stack reads the same Markdown: headings (`#` to `######`), paragraphs, bullet and numbered items (indented two spaces per level), quotes (`> `, `>>` inside a quote), scene breaks (`***`, `---` or `___` alone on a line; a line of `---` under text is a break too, never an underlined heading), hard line breaks (two spaces or a backslash at a line's end), `**bold**`, `*italic*`, `***both***`, `` `code` ``, code blocks between three backticks, and links `[text](https://...)`. HTML is text. `conformance/markdown.json` gives each case.
- It reads like a page: room around the text, a column no wider than a reading width, from the left, headings by their level in the theme's heading font and size with space above them, quotes set in with a bar, a scene break as a short line in the middle.
- A link opens in the person's browser when it is an `http`, `https` or `mailto` address; with any other address only its text shows.
- The part wraps its text and scrolls when it is longer than the part.
- To write Markdown with its preview, a memo has `markdown`: `memo name manuscriptBox at 1,1 span 12 rows 14 bind chapter.text markdown`. Write and Preview stand on its top (Clarity's words `markdownWrite` and `markdownPreview`, which a project with such a memo gives); Write is the memo, its Markdown colored as it is written in the text font, Ctrl+B and Ctrl+I making the selection bold and italic; Preview shows the text as a markdown part does. It takes `code` no more: it colors its Markdown itself.

## 3. Modules

Every other `.clarity` file in the project folder is a module. It starts with `module`, then the modules it uses, then its declarations:

```clarity
module sales

use customers

entity Order
    customer: ref Customer
```

A module of entities that live in another database:

```clarity
module erp

use customers
source erp

entity Invoice
    number: text(20), unique
    customer: ref Customer
```

A reference to an entity in another database is stored as its id, without a foreign key, so the database cannot stop a referenced record from being deleted.

- `module name` is the first line of the file (comments and blank lines may come before it).
- `use other` makes the declarations of module `other` visible in this file. Without it, a name from another module is an error that tells you which `use` is missing.
- `source name` places the entities of this module in that data source of the project. Without it, they live in the first data source. In a SQL database (sqlite, postgres, mysql or mssql) Clarity stores them; in a web API (`rest`) the service keeps them, so no migration covers them and `find`, `save` and `delete` do not reach them: the module's actions read and send them with `http ... from name` (see [Calling web APIs](#calling-web-apis)). One module has at most one `source` line.
- A window, a view and the app are no declarations of a module: each is a file of its own in the module folder's `design` folder, whose root names the module and the page or app (see [5](#5-windows) and [7](#7-the-app-and-its-frame)). Its code is a `code Name` block in the module.
- A name is declared once per module. Two modules may declare the same name, but a file that sees both (its own module and the modules it uses) cannot use that name: it is ambiguous, and the checker says so.

The declarations a module can hold:

| Declaration | Section |
|---|---|
| `enum Name` | [4](#4-data-enums-and-entities) |
| `entity Name` | [4](#4-data-enums-and-entities) |
| `settings` | [Settings](#settings), one per project |
| `code Name` | [8](#8-code-behind) |
| `function name(...)` and `action name(...)` | [9](#9-functions-and-actions) |
| `engine Name`, `engine Name from "file.wasm"` | [9](#engines) |

## 4. Data: enums and entities

### Enums

An enum is a fixed list of values. Each value can have a label; without one, the label is derived from the name.

```clarity
enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness
```

### Entities

An entity is a kind of record that is stored in a data source. It is the data dictionary: everything about a field is said once, here, and the windows, the database schema and the validation all follow from it.

```clarity
entity Customer
    name: text(80), required, label @customerNameLabel
    email: email, unique, label @customerEmailLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vatNumber: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    creditLimit: money, min 0, default 0, label @customerCreditLimitLabel
    active: bool, default true, label @customerActiveLabel
    since: date, label @customerSinceLabel
    notes: text, help @customerNotesHelp, label @customerNotesLabel
```

A field is `name: type`, followed by modifiers separated by commas. Every record also has an `id`, which Clarity adds itself. Code can read it, to tell two records apart (`other.id != loan.id`); only `save` sets it, and a new record has id 0.

**Field types**

| Type | Holds |
|---|---|
| `text` | text of any length |
| `text(n)` | text of at most `n` characters |
| `integer` | a whole number |
| `decimal(p, s)` | a decimal number with `p` digits, `s` of them after the point |
| `money` | an amount of money, exact |
| `bool` | true or false |
| `date` | a calendar date |
| `datetime` | a date with a time |
| `duration` | a length of time in whole seconds, shown and typed as `3:45` or `1:02:03` everywhere |
| `image` | a picture (png, jpg, gif, bmp or webp), kept in the database, or with `store` as a file; see [Pictures in records](#pictures-in-records) |
| `file` | any file, with its name, kept in the database, or with `store` as a file; see [Files in records](#files-in-records) |
| `email` | an e-mail address |
| `url` | a web address |
| `color` | a color, kept as text `#rrggbb`: `default "#3b82f6"`; code sets it as such text |
| `id` | the id of a record |
| an enum name | one value of that enum |
| `ref Entity` | a reference to a record of another entity |
| an entity name | in an entity of a web API, a record of that entity held inside this one, as the service's JSON nests it: `owner: User`; see [Calling web APIs](#calling-web-apis) |
| `list<Entity>` | the records of another entity that belong to this one; in an entity of a web API, held inside it |
| `list<text>`, `list<integer>`, `list<decimal>`, `list<date>`, `list<Enum>` | a list of values, kept in one column; see [Lists in records](#lists-in-records) |

**Modifiers**

| Modifier | Meaning |
|---|---|
| `required` | The field must have a value. Windows mark its label, and `save` refuses a record while the field is empty (no value, or empty text; 0 and false are values) in Clarity's word `fieldRequired`, "{field} is required" with the field's label, after `before save`, so nothing is kept. The refusal shows at the window's input bound to that field, else as an error message; code can catch it. |
| `unique` | No two records have the same value. Empty text counts as no value, so several records can leave a unique text field empty. |
| `search` | The field is in its record's full-text index, which `where a matches x` looks in. For text (also `email` and `url`) and lists of text; see [Full-text search](#full-text-search). |
| `min value`, `max value` | The lowest and highest value allowed. |
| `pattern "regex"` | Text must match the regular expression. |
| `default value` | The value of a new record: a number, a string, `true`, `false` or an enum value. A date is written `"2026-10-17"` and a date and time `"2026-10-17T09:30:00"` (seconds may be left out); the checker refuses any other. |
| `label @key` | The label people see, by its key. Every field has one. |
| `help @key` | A help text, by its key, shown next to the field's input in windows. |
| `format name` | How the value is shown, by name; see [Formats](#formats). |
| `owned` | For a `list<...>`: the listed records live and die with this record. |
| `hashed` | For text of an entity: assigning stores the hash the project's sign-in names for it (Argon2id when none does); code never reads it and no part shows it. Text without a length or of at least 128 characters; no `search`, `default`, `pattern`, `min`, `max` or `format`. See [Signing in](#signing-in). |
| `cascade` | For a `ref Entity`: deleting the record it refers to deletes this one first, with its own cascades and data events, all in the transaction of that delete. When one of them refuses, nothing is deleted. |
| `computed` | The value is computed, not entered. Accepted, but not used by the generated application yet. |
| `column "Name"` | The existing column the field is stored in, for an entity of an external database. |
| `store name` | For an `image`: its pictures are files in that file store of the project settings, not in the database. |

Each modifier appears at most once per field. The checker reports defaults and limits that do not fit the type, such as `min "x"` on a number, or a default that is not one of the enum's values.

**Versions: a save never overwrites someone else's change**

Every record of an entity in a database Clarity owns carries a version, a whole number in a column of its own, `row_version`, which Clarity keeps: 1 for a new record, one more with each save. Code never sets it; `find` reads it with the record. A save writes only when the version is still the one it read, and a delete is checked the same way. When someone (another person, window or background job) saved the record since, the save is refused and changes nothing: the handler stops and the person sees Clarity's word `savedSince`, "Someone else changed this record since you opened it. Reload it to see their change." `reload customer` reads the record as it is stored now, and `save customer anyway` writes regardless of the version (see [Statements](#statements)). One record is one unit: fields are never merged.

The column comes from a migration step (`add version Customer`, or `version` in an `add entity`; see [Migrations](#migrations)); until that step exists, saves of the entity are not checked. An entity of an external database names its own column with `version "Column"`; see [External databases](#external-databases).

The text of the refusal can be the entity's own, by a key, written on the entity:

```clarity
entity Customer
    name: text(80), label @customerNameLabel
    changedBy: ref Person, label @customerChangedByLabel
    savedSince @customerChangedSince
```

Its text may use the slots of the record as it is stored now: `current.` and a field (`{current.name}`), its `{current.id}`, and a field of a record it refers to (`{current.changedBy.name}`), as in `customerChangedSince "{current.changedBy.name} changed this customer since you opened it"`. Who changed a record is the project's choice: Clarity adds no such column, and a `before save` that sets `customer.changedBy = me` keeps it (see [Data events](#data-events) and [Signing in](#signing-in)). A record type never has `savedSince`, as it is never stored.

### Pictures in records

An `image` field holds a picture, or none. Without `store`, the picture is kept in the database. With `store`, it is a file in a file store, a folder that the project settings name, and the column holds the file's name there:

```clarity
entity Album
    cover: image, help @albumCoverHelp, label @albumCoverLabel

entity Artist
    photo: image, store photos, label @artistPhotoLabel
```

```clarity
// records.clarity
    filestore photos "data/photos"
```

- An image part shows it: `image name cover at 1,1 span 4 rows 6 bind album.cover`. It shows its `noPicture` text while there is none, and, while the pointer is on it, a button to load a picture from a file and one to remove it. A label or help text can be for it.
- Reading records never carries their pictures, so a list of thousands stays light; the screen fetches each picture on its own when it shows.
- A picture is a png, jpg, gif, bmp or webp file, told by its first bytes; any other file is refused.
- In a file store, a picture's file is named after the SHA-256 of its bytes, so the same picture is kept once. Files are never deleted: a store can be a folder of photos that are not the application's alone.
- In code, a picture is compared with `none` and set to `none`: `if album.cover != none`, `album.cover = none`. It is set to another record's picture as well, `album.cover = other.cover`, between the database and a file store too: the picture's bytes go along, and saving keeps them where the field keeps its pictures. A query can find the records with or without one: `where a.cover == none`.
- A table or list does not show a picture column; an image part does.
- Moving a field's pictures between the database and a file store, or between `image` and another type, is a change that can lose data: pictures do not move by themselves.

### Files in records

A `file` field holds any file a record keeps, such as a manual, a contract or a spreadsheet, with its name, or none:

```clarity
entity Product
    manual: file, label @manualLabel
    sheet: file, store documents, label @sheetLabel
```

- **Where it is kept.** Without `store`, the file is in the database: its bytes in the field's column and its name in a column beside it, `manual_name`, which migrations add, rename and remove with the field. With `store`, it is a file of that file store, in a folder named by its contents and under its own name (`3fa9c1.../datasheet.pdf`); the column holds that path, and the name column is there too. A field cannot become a file or stop being one; remove it and add a new field.
- **The file part** shows the file's name, or Clarity's word `noFile`, with buttons to choose a file, open it in the program the system uses for its kind, save a copy, and remove it: `file name manualBox at 1,1 span 8 bind product.manual`. Choosing and removing are its `change`. A copy goes to the person's Downloads folder under the file's name (with ` (2)` and on when that name is taken there), and the part says so in Clarity's word `fileCopySaved`. Its buttons are Clarity's words `chooseFile`, `openFile`, `saveFileCopy` and `removeFile`.
- **Reading records never carries the file itself,** only its name; the file comes on its own when it is opened or copied.
- **In code,** `product.manual.name` and `product.manual.extension` read its name and extension, a file is compared with and set to `none`, set to a file of a store, as `files f in documents` finds them (see [Files](#files)), and set to another record's file, `product.manual = other.manual`, which keeps its name. A file cannot be unique or have a default, and a table or list does not show a file column; a file part does.

### Lists in records

A field can hold a list of values: tags, the formats an album came out in, the days a show was played.

```clarity
entity Album
    tags: list<text>
    formats: list<Format>
    plays: list<date>
```

- **The item types** are `text`, `integer`, `decimal`, `date` and an enum. A list is kept in one column as JSON (`["jazz","live"]`) and starts empty; migrations add it as an empty list. A field cannot become a list or stop being one; remove it and add a new field. A list cannot be unique, and has no default, `min` or `max`.
- **The tags part** shows the items as chips, each with a button that removes it, and a place to add one: `tags name tagsBox at 1,1 span 6 bind album.tags`. Text and numbers are typed and taken on Enter or when the box is left; a typed item is trimmed, an item the list has is not added twice, and text that is no number stays as typed and is marked. A date is picked; an enum's values are picked from the ones the list has not. Adding and removing are its `change`. Its words are Clarity's `addTag` and `removeTag`.
- **A table** shows a list column as its items joined by a comma and a space, an enum's values by their labels. It is not edited in the cell.
- **In code** a list field is a list of values like any other: `for play in album.plays`, `count(album.tags)`, `album.tags[1]`, a new list `album.formats = [Vinyl, Cd]`, and `album.tags contains "live"` for whether it holds an item. A query finds the records whose list holds an item, exactly: `find a in Album where a.tags contains "live"` finds an album tagged `live`, not one tagged `live music`. Each database does it with its own JSON functions.
- Every stack keeps and reads a list alike: the cases are in `conformance/list-fields.json`.

### Formats

Dates, times, numbers and money show the way the person using the application writes them: the locale comes from their system (or, where it names none, the project's `language`). The value stored is the same in every locale.

| Type | Shown by default | `format` |
|---|---|---|
| `integer` | with thousands separators: `1,234,567`, `1.234.567` | `plain`: without them, for codes and years |
| `decimal(p, s)` | with `s` decimals: `-1,250.50`, `-1.250,50` | `plain`: without thousands separators; `percent`: `0.25` shows `25%` |
| `money` | with the project's currency and its decimals: `€1,250.50`, `€ 1.250,50`, `1.250,50 €` | `plain`: the amount without the currency |
| `date` | in digits: `10/17/2026`, `17-10-2026`, `17.10.2026` | `short` (the default), `long`: `17 October 2026`, `iso`: `2026-10-17` everywhere |
| `datetime` | the date and the time: `10/17/2026 9:30 AM`, `17-10-2026 09:30` | `short`, `long`, `iso` as for dates |

```clarity
entity Invoice
    issued: date, format long
    rate: decimal(5, 4), format percent
    year: integer, format plain
```

- A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5 and a point is a thousands separator. The currency sign, the percent sign and thousands separators may be typed or left out. Text that is no number marks the input and keeps the last value.
- Every stack shows the same thing: the cases in `conformance/formats.json` are what each renderer must show and read.
- Values in text that is shown follow the locale when they are in slots: `message @savedOn(day = order.placed)`. See [Texts by key](#texts-by-key).

### Migrations

A database follows the dictionary through migration steps. Every SQL data source has its own steps, in a folder of the project's `migrations` folder named after the source and numbered in the order the steps run: `migrations/main/0001.clarity`, `migrations/main/0002.clarity`. A step changes only the entities of its source. A step holds one `migration` block with a title and its changes:

```clarity
migration "Phone and VAT"
    rename field Customer.vatNumber to vat
    add field Customer.phone: text(20)
    change field Order.number: text(20)
    add unique Customer.email
```

| Change | Does |
|---|---|
| `add entity Name` with field lines below it | creates the records of an entity |
| `remove entity Name` | removes an entity and its records |
| `rename entity Old to New` | renames an entity; its records stay |
| `add field Entity.name: type, modifiers` | adds a field; existing records get its `default`, or the empty value of its type |
| `remove field Entity.name` | removes a field and its values |
| `rename field Entity.old to new` | renames a field; its values stay |
| `change field Entity.name: type` | changes a field's type, converting the values |
| `add unique Entity.name`, `remove unique Entity.name` | makes a field unique, or not |
| `add search Entity.name`, `remove search Entity.name` | puts a field in its entity's full-text index, or takes it out |
| `map table Entity to "name"` | stores the entity in a table of another name (renames the table); `to default` goes back to the entity's name |
| `map key Entity to "name"` | names the key column (renames it); `to default` is `id` |
| `map column Entity.field to "name"` | stores a field in a column of another name (renames the column); `to default` is the field's name in snake case |
| `add version Entity` | gives the entity's table its version column, `row_version`, at 1 for the records already there, so saves of it are checked (see [Entities](#entities)) |

Fields in a step are written as in an entity. Only what is stored counts: the type, `unique`, `column`, and `default` for the records that already exist. Labels, help texts and checks such as `min` belong to the dictionary alone. An added entity can name its table and key with `table` and `key` lines, as in the dictionary; renaming a field whose column is mapped changes nothing in the database. A line `version` in an added entity gives its table the version column from the start. Clarity writes it in every entity it adds to a database it owns, and proposes `add version Entity` for each table that does not have the column yet, one line per entity; the baseline step of Take ownership leaves it out, as the tables it describes have no such column, and the next step adds it.

```clarity
migration "Versions"
    add version Customer
    add version Order
```

- **Replaying the steps** in order gives the schema as last migrated. Each step must fit the schema the steps before it made; one that renames a field that does not exist is an error.
- **Pending changes** are what the dictionary has and the steps do not. The checker reports them as warnings on the entity, and the generator refuses to write the application until a step covers them. A missing version column alone does not hold the application back: the table is written as it is and its saves stay unchecked until the step that adds the column exists, which the checker's warning says.
- **Writing a step.** The Project tab lists the pending changes and writes them as the next step; `clarity migrate shop.clarity` does the same from the command line. Clarity proposes a rename when a field disappears and a field of the same type appears in the same entity, or an entity disappears and one with mostly the same fields appears. You confirm it or take it as a removal and an addition. Changes that can lose data (removing, changing a type) need a confirmation.
- **Running.** The generated application applies the steps its database does not have yet when it starts, each in a transaction where the database allows it (see Databases below). On SQLite, which does not enforce references while it builds a table again, it checks the references between records before a step is kept.
- **A backup first.** Before it applies steps to an SQLite database that already has a schema, the application backs it up beside it, in `backups/` (`data/backups/shop-20261105T205905Z-before-migration-0006.db`, the time in UTC), and keeps the newest ten of those. When it cannot back up, it does not migrate. The Database section of the Project tab takes a backup by hand and restores or deletes one; `clarity data backup shop.clarity`, `clarity data backups shop.clarity` and `clarity data restore shop.clarity <file>` do the same from the command line. A restore first backs up what it replaces. Server databases are backed up by their administrator for now (docs/BACKUP-PLAN.md).
- **A step that ran never changes.** To undo it, write a new step.

### Databases

The same entities and steps work on every SQL database. The generated application carries a driver for each kind of database its data sources are, and writes each step in that database's own SQL.

```clarity
datasource main postgres "env:SHOP_DATABASE"
```

| Kind | Where the database is | Example |
|---|---|---|
| `sqlite` | a file, relative to the project's folder | `"data/shop.db"` |
| `postgres` | a connection string or URL | `host=db.example.com user=shop dbname=shop`, `postgresql://shop@db.example.com/shop` |
| `mysql` | a URL | `mysql://shop@db.example.com:3306/shop` |
| `mssql` | an ADO.NET connection string | `server=tcp:db.example.com,1433;database=shop;user=shop;TrustServerCertificate=true` |

A connection that needs a password is written as `"env:NAME"`: the generated application reads it from that environment variable when it starts, so the password is never in the project. It may also be a [secret setting](#secret-settings), `"setting:name"`, which the system's keychain keeps.

How fields are stored:

| Type | SQLite | PostgreSQL | MySQL | SQL Server |
|---|---|---|---|---|
| `text(n)` | TEXT | VARCHAR(n) | VARCHAR(n); TEXT above 768 | NVARCHAR(n); NVARCHAR(MAX) above 4000 |
| `text`, `url` | TEXT | TEXT | TEXT | NVARCHAR(MAX) |
| `email` | TEXT | VARCHAR(320) | VARCHAR(320) | NVARCHAR(320) |
| `color` | TEXT | VARCHAR(7) | VARCHAR(7) | NVARCHAR(7) |
| `file` | BLOB, and TEXT for its name | BYTEA, VARCHAR(255) | LONGBLOB, VARCHAR(255) | VARBINARY(MAX), NVARCHAR(255) |
| `integer`, `id`, `ref` | INTEGER | BIGINT | BIGINT | BIGINT |
| `decimal(p, s)` | TEXT | NUMERIC(p,s) | DECIMAL(p,s) | DECIMAL(p,s) |
| `money` | TEXT | NUMERIC(19,4) | DECIMAL(19,4) | DECIMAL(19,4) |
| `bool` | INTEGER | BOOLEAN | BOOLEAN | BIT |
| `date` | TEXT | DATE | DATE | DATE |
| `datetime` | TEXT | TIMESTAMP | DATETIME | DATETIME2 |
| an enum | TEXT | TEXT | VARCHAR(100) | NVARCHAR(100) |

SQLite keeps decimals and dates as text, so no precision is lost. The key is an integer the database counts up, and a `ref` field gets a foreign key when its entity is in the same database. The version column is stored as an integer, `row_version INTEGER NOT NULL DEFAULT 1` on SQLite and `BIGINT NOT NULL DEFAULT 1` on the others.

What differs between the databases:

- **Unique text.** MySQL and SQL Server index text up to a length (768 and 850 characters), so a `unique` text field there needs a maximum length that fits, such as `text(200)`; the generator says which field. Empty text is never a duplicate: SQLite, PostgreSQL and SQL Server leave it out of the index, MySQL indexes it as no value.
- **Changing a type.** SQLite builds the table again; the others add a column of the new type, copy the values over converted, and drop the old one. Empty text that becomes a number or a date gets the field's default.
- **A failed step.** SQLite, PostgreSQL and SQL Server undo the whole step. MySQL cannot undo changes to tables, so it keeps the statements of the step that ran before the failure; the error says how many.
- **Comparing text.** `==` and `contains` follow the database: MySQL and SQL Server usually ignore case in `==` too, and sort text by their own rules.
- **Full-text search.** Each database keeps its own index of the fields marked `search`: SQLite an FTS5 table that triggers keep (`album__search`), PostgreSQL a GIN index on `to_tsvector('simple', ...)`, MySQL a FULLTEXT index without stopwords, and SQL Server none, as its full-text search is seldom installed: it looks with LIKE at the start of each word, reading every record. A step that changes an entity builds its index again. Whether accents count follows the database: MySQL usually ignores them, the others do not.
- **Baseline steps**, Import and the check of external tables are for SQLite only for now. An external PostgreSQL, MySQL or SQL Server database can be mapped by hand and is read and written as it is.

### External databases

A database that already exists, and that Clarity does not own, is a data source marked `external` in the project file:

```clarity
datasource erp sqlite "legacy/erp.db" external
```

Its entities live in a module with `source erp`, and they map onto the tables and columns that are there:

```clarity
entity Invoice
    table "tblInvoice"
    key "InvoiceID"
    invNo: text(20), required, unique, column "InvNo"
    custId: ref Customers, column "CustID"
    amount: decimal(18, 6), column "Amount"
```

- `table` names the existing table; without it, the table is the entity's name in snake case (`invoice`).
- `key` names the integer column that identifies a record; without it, that column is `id`. Tables whose key is not one integer column cannot be used yet.
- `column` names the existing column of a field; without it, the column has the field's name.
- `version "RowVersion"` names the column that holds each record's version, a whole number the database keeps as Clarity's own `row_version`: a save writes only when it is still the one read, and adds one. `version none` says the entity's saves are not checked. Without either line the checker warns, "Invoice has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none", so an unchecked entity is a choice and not an accident; a `readonly` source needs neither. `version` is only for external databases: a database Clarity owns has `row_version`, a file source's file is its own version, and a web API has no such column.
- Two fields cannot share a column, no field can use the key column, and the version column is neither the key nor a field's column. The mapping is meant for existing tables; it stays when Clarity takes ownership (below).

Clarity reads and writes the records of an external database, but never changes its tables: it writes no migration steps for it, and the generated application never migrates it.

- **Import.** The Import button of an external SQLite source on the Project tab, or `clarity import shop.clarity erp`, writes the chosen tables as a new module of mapped entities. Names follow Clarity's style (`tblCustomers` becomes `Customers`, `CustName` becomes `cust_name`), column types become field types, foreign keys become `ref` fields when both tables are imported, `NOT NULL` becomes `required` and a unique index `unique`. Binary columns are left out. An integer column named `row_version`, as in a table Clarity made, becomes the entity's `version "row_version"`; any other version column is named by hand, never guessed.
- **Checking.** Each time the project loads, Clarity compares the entities with the tables of the database. A missing table or column, the version column included, is an error; a column whose type does not fit the field, and a required column that no field fills (so new records cannot be saved), are warnings. When the database cannot be read, a warning says its tables were not checked.
- **Read-only.** With `readonly` on the data source, `save` and `delete` of its records are errors.
- **Take ownership.** The Own button of an external source on the Project tab, or `clarity own shop.clarity erp`, makes Clarity the owner. It writes the source's first step, a `baseline`, which describes the mapped tables as they are and creates only what is missing, so it changes nothing in the database it was taken from (and a new, empty database gets the tables). Then it removes `external`. From there on the dictionary drives the tables through ordinary steps, and the mapped names stay: `table`, `key` and `column` work on owned entities too.

### Settings

What each person sets for themselves, such as the language, the theme, whether the toolbar shows or how many rows a page holds, are settings: one `settings` block per project, in any module.

```clarity
settings
    language
    theme
    showToolbar: bool, default true, label @showToolbarSetting
    exportFolder: text
    pageSize: integer, min 10, max 200, default 50
```

- **Fields.** A setting is written as a field of an entity: a name, a type and the modifiers `default`, `min`, `max`, `label`, `help` and `format`. Its type is a plain one: text, a number, money, bool, a date or moment, a duration, email, url, color, an enum, a secret (see [Secret settings](#secret-settings)) a folder (see [Folder settings](#folder-settings)) or a reference to a record (`openProject: ref Project`). A list, a picture or an id is no setting.
- **References.** A `ref` setting keeps the record's id, and is empty until the person picks one. Code reads it as a `ref` field: `settings.openProject != none`, `let open = settings.openProject`, `settings.openProject = project`. A dropdown bound to it offers the entity's records. A kept id that is not a whole number above 0 is dropped; when its record is deleted, loading it fails as for any reference, so code that deletes the record clears the setting.
- **Clarity's own.** `language` and `theme` take no type: they are the language and the theme shown, labeled with Clarity's words `languagePicker` and `themePicker`. Setting either switches the language or the theme, as the pickers of the click targets `language` and `theme` do, and those pickers change the same settings.
- **For AI agents.** `agentReadsWhileClosed` and `agentChangesWhileClosed` take no type either: bools, labeled with Clarity's words `agentReadsWhileClosedSetting` and `agentChangesWhileClosedSetting`, that allow AI agents to read and to change through the app's MCP service while the app is closed (see [Commands](#commands)). They start as the project file's `agents while closed` line says, off without it, and belong to an app with a command for agents.
- **Kept per person.** Settings are never in the database. Each person's are kept on their machine, as `settings.json` in a folder named after the application, in the folder for application settings of their system (`%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` or `~/.config` elsewhere). A setting not kept yet has its `default`; a kept value that no longer fits (a setting removed, an enum value gone, a number past its ends) is dropped.
- **In every window.** Every window and the app have `settings` as their data without a `Data` element. A part binds `settings.pageSize`, a form binds `settings`, and code reads and sets `settings.pageSize`. A change takes effect at once and is kept, and every window sees it. A dropdown bound to `settings.language` offers the project's languages by their own names, and one bound to `settings.theme` the offered themes by their titles.
- **A settings window.** New window offers "Settings window" when the project has settings: a dialog with a labeled form of every setting and a Close button.

The name `Settings` is theirs: no entity can have it, and no window can have other data called `settings`.

#### Secret settings

A token, a key or a database's connection is a setting of type `secret`:

```clarity
settings
    apiToken: secret, label @apiTokenSetting
```

- **Kept apart.** A secret is kept in the system's protected store, per application and per person: the Credential Manager on Windows, the Keychain on macOS, the Secret Service (such as GNOME Keyring or KWallet) elsewhere. It is never in `settings.json`.
- **Only a password input shows it.** A text box or a text part bound to it is an error; a settings window gives it a password input. It has no default.
- **In code it is text.** `settings.apiToken = value` keeps it, and `settings.apiToken = ""` forgets it.
- **Only in settings.** An entity's field cannot be a secret.
- **Signing and connecting.** The project file names it as `"setting:apiToken"`: a web API signs its calls with it (`auth bearer "setting:apiToken"`), and a server database connects with it (`datasource main postgres "setting:shopDb"`). While it is not kept, the database does not open: the application then shows the window where the secret is entered, saying why, and connects once it is kept and that window closes. The app's `on unconnected` handler can show something else instead (see [App settings](#app-settings)). A secret connection needs one of the two.

#### Folder settings

The folder a file store keeps its files in can be each person's own choice, chosen while the application runs: a setting of type `folder`.

```clarity
settings
    photosFolder: folder, default "photos", label @photosFolderSetting
```

```xml
<!-- records/design/Preferences.cxaml -->
<Dialog class="records.Preferences" title="@preferencesTitle">
  <Folder name="photosBox" at="1,1" span="6" bind="settings.photosFolder" />
</Dialog>
```

```clarity
// records.clarity, the project file
    filestore photos "setting:photosFolder"
```

- **Kept with the settings,** in `settings.json`, like any other setting. Until the person chooses one, the store is in its `default`; without a default, in a folder named after the store.
- **A relative folder** is in the folder the settings file is in (such as `~/.config/records/photos`); an absolute one is used as it is.
- **Read at every use.** A folder chosen while the application runs is where the next file goes, at once. Files already in the old folder stay there: moving them is the person's choice.
- **The folder part shows it:** a text field with the path and a button, in Clarity's word `chooseFolder`, that opens the system's folder picker. In a browser there is no picker, so the path is typed. Only a folder part shows a folder setting.
- **In code it is text:** `settings.photosFolder = "/mnt/photos"`.
- **Only in settings,** and only a folder setting can be a store's `setting:name`; anything else is an error that says to add it as `name: folder`.

## 5. Windows

A window is a file of its own, `design/<Name>.cxaml` in the module's folder: XML whose root element says what the page is and names its module and page, as Avalonia's `x:Class` does. Its attributes are the window's settings; inside are its data, then its parts, which sit on a snap grid.

```xml
<!-- shop/design/CustomerForm.cxaml -->
<?xml version="1.0" encoding="UTF-8"?>
<Window class="shop.CustomerForm" title="@customerFormTitle" onreturn="windowReturned">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="CustomerFormView" />
  </Content>
</Window>
```

```xml
<!-- shop/design/CustomerFormView.cxaml -->
<?xml version="1.0" encoding="UTF-8"?>
<View class="shop.CustomerFormView">
  <Data name="customer" type="Customer" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
  <Textbox name="name" at="4,1" span="6" bind="customer.name" />
  <Table name="orderTable" at="1,3" span="12" rows="6" bind="orders" columns="number, total" />
  <Button name="saveButton" at="11,10" span="2" caption="@saveButtonCaption" default="true" onclick="saveClicked" />
</View>
```

```clarity
// shop/shop.clarity: the code of both
code CustomerForm
    handler windowReturned
        reload orders

code CustomerFormView
    handler saveClicked
        save customer
```

- **One file per page.** The file is named after the page, `CustomerForm.cxaml`, and lives in `design/` beside the module's `.clarity` files. The app has one too, `design/Shop.cxaml` (see [7](#7-the-app-and-its-frame)).
- **The root names it.** `class="shop.CustomerForm"` is the page `CustomerForm` of module `shop`; its code is the block `code CustomerForm` of that module. A class whose module the project does not have is an error, and so is a `page` block left in a module's `.clarity` file.
- **The code stays in the module.** Handlers, functions and actions are Clarity, in the module's `.clarity` files; the `.cxaml` file names them (`onclick="saveClicked"`) and holds no code.
- **Window, content, view.** A window shows its parts in a view: its `Content` part lists the views it can show, and the view holds the parts, data and handlers (see [Views](#views)). New window in the designer makes a window this way, with an empty view `<Name>View`.
- **Escaped as XML.** A value with `<` or `&` writes them as `&lt;` and `&amp;`, as in `type="list&lt;Order&gt;"`.
- **The designer writes it.** The designer reads and writes these files; the Text tab of the inspector shows the file as it is. A part writes only what differs from its kind's defaults.

### Kinds

The root element is the page's kind:

| Root | Is |
|---|---|
| `<Window>` | a window of its own, which can have a menu, toolbar, ribbon, sidebars and a footer |
| `<Dialog>` | a modal window; `gives="Type"` closes it with a value of that type (see [Asking the person](#asking-the-person)) |
| `<View>` | a piece of a window, shown in a content (see [Views](#views)) |
| `<Splash>` | the splash screen the app shows while it starts (see [Splash screen](#splash-screen)) |
| `<Toolbox>` | the tools a canvas or memo shows, designed as a view (see [Canvas modes](#canvas-modes)) |
| `<SignIn>` | the sign-in window the program shows before the main window when people sign in; nothing opens it, it has no frame and no `allow` (see [Signing in](#signing-in)) |

A window, dialog or view may say who may open it: `allow="me.role == Admin"`, a condition on `me`.

Only a `Window` has frame regions of its own, written as elements in its file: `<Menu>`, `<Toolbar>`, `<Ribbon>`, `<Sidebar>` and `<Footer>`.

### Splash screen

An application may show a splash screen while it starts; it is optional, and an app without one opens its main window as before. The splash is a page of the kind `Splash`, designed as any page is (a picture, the name, a progress bar), and the app names it with how long it shows at least, `splash` and `splashFor` (seconds) on its root:

```xml
<!-- shop/design/Shop.cxaml -->
<App class="shop.Shop" main="Main" splash="Welcome" splashFor="2" onstart="loadCatalogue">
</App>
```

```clarity
// shop/shop.clarity
code Shop
    handler loadCatalogue
        splash.status = @loadingCatalogue
```

```xml
<!-- shop/design/Welcome.cxaml -->
<?xml version="1.0" encoding="UTF-8"?>
<Splash class="shop.Welcome" width="560" height="290" style="splashBack">
  <Image name="logo" at="1,1" span="4" rows="5" source="logo.png" />
  <Heading name="titleHeading" at="5,2" span="8" rows="2" caption="@shopTitle" style="splashTitle" />
  <Label name="tagline" at="5,4" span="8" caption="@shopTagline" style="splashTagline" />
  <Busy name="status" at="5,5" span="8" caption="@welcomeStarting" style="splashStatus" />
</Splash>
```

- It shows first, while the app's `on start` runs, and goes once the main window shows, but not before the time the line gives (0.1 to 30 seconds; without `for`, as soon as the main window shows).
- `splash.status = text` in code says what loads: the caption of the splash page's part named `status`, a `label`, or a `busy` part that spins while it shows. Code anywhere may set it; it shows while the splash does.
- On the desktop (every stack, through Clarity.UI) it is a borderless window in the middle of the screen, above the others. In the browser it fills the page until the main window takes its place.
- As in Avalonia, the generated markup says it: the generated program's window file for the splash (`windows/Welcome.cxaml`) declares its window, `<Window name="Welcome" width="480" height="280" decorations="none" startup="centerScreen" resizable="false" topmost="true" taskbar="false" frame="none">` (Avalonia's Width, Height, SystemDecorations, WindowStartupLocation, CanResize, Topmost and ShowInTaskbar, and none of the app's menu, toolbar or ribbon, as an Avalonia window holds only what its XAML holds), and the app's markup names it, `<App name="Shop" splash="Welcome" splashFor="2">`. A splash page without `width` and `height` is 480 by 280.
- The application shows and closes it: nothing opens it (`on click -> Welcome` is refused), it has no events of its own, and it cannot be the main window. A splash page no app names is warned about.
- When the databases did not open, the app starts without its splash and shows what keeps the connection.
- The app's splash may double as the sign-in window: with a sign-in part on it, or the strategies' inputs and `-> signin <strategy>` buttons, it stays open once `on start` is done until someone signs in, then the main window opens (see [Signing in](#signing-in)).

### Views

A window is a composition of views. A view is designed as a window is (its parts on its grid, its data, its code), in its own file with a `<View>` root, without a frame of its own. It shows where a window's `Content` part names it, with a `<View name="..." />` line. A content shows one of the views it lists at a time; code picks another with `main.view = OtherView`. A slide-out, tab, step or section shows a view through a content inside it. A content holds no parts of its own besides its view lines.

```xml
<!-- design/Workspace.cxaml -->
<Window class="modelling.Workspace" title="@workspaceTitle">
  <Content name="main" at="1,1" span="12" rows="20">
    <View name="WorkspaceView" />
  </Content>
</Window>
```

```xml
<!-- design/WorkspaceView.cxaml -->
<View class="modelling.WorkspaceView">
  <Data name="picked" type="text" />
  <Tree name="classTree" at="1,1" span="3" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" onselect="classPicked" />
  <Slideout name="classPanel" at="4,1" span="6" rows="12" caption="@classCaption" slideWidth="520" pinnable="true">
    <Content name="classContent" at="1,1" span="6" rows="12">
      <View name="ClassEditor" />
    </Content>
  </Slideout>
</View>
```

```xml
<!-- design/ClassEditor.cxaml -->
<View class="modelling.ClassEditor">
  <Data name="className" type="text" />
  <Label name="nameLabel" at="1,1" span="2" caption="@nameLabel" for="nameBox" />
  <Textbox name="nameBox" at="3,1" span="4" bind="className" />
  <Button name="saveButton" at="5,2" span="2" caption="@saveCaption" onclick="saveClicked" />
</View>
```

```clarity
code WorkspaceView
    handler classPicked
        className = classTree.selected.label
        classPanel.visible = true

code ClassEditor
    handler saveClicked
        // works on its own parts and data
```

- Where it shows, the view's parts, data and handlers join the window's: the window's code reaches them by name, so the window sets what the view shows (`className = ...`). The view's own code reaches only its own parts and data, so one view can show in several windows.
- A window shows a view once; the view's part and data names differ from the window's.
- Only a view tells; the content that shows it hears only what the view's file says it tells, and the handler takes the tell's values.
- In the IDE views have their own section in the project tree, with **+ New view**, and are designed on their own as windows are. The designer shows the content as written; the Preview shows it with the view in it.

A view tells its window what happened, as an engine beside the screen does: its file says `<Tells message="saved" />` (with values, `<Tells message="saved(name: text)" />`), its code says `tell saved()`, and the window hears it on the view line that shows it, `<View name="RecipeEditor" onsaved="recipeSaved" />`, with a handler that takes those values. A tell the window does not hear does nothing.

```xml
<!-- design/CookbookView.cxaml -->
<View class="kitchen.CookbookView">
  <Data name="recipes" type="list&lt;Recipe&gt;" />
  <Content name="recipeContent" at="5,1" span="8" rows="14">
    <View name="RecipeEditor" onsaved="recipeSaved" />
  </Content>
</View>
```

```xml
<!-- design/RecipeEditor.cxaml -->
<View class="kitchen.RecipeEditor">
  <Data name="dish" type="Recipe" />
  <Tells message="saved" />
  <Button name="saveButton" at="5,8" span="2" caption="@saveCaption" onclick="saveClicked" />
</View>
```

```clarity
code CookbookView
    handler recipeSaved
        recipes = find r in Recipe order by r.name

code RecipeEditor
    handler saveClicked
        save dish
        tell saved()
```

An accordion, tabs or a wizard can be a composition of views too, each section, tab or step holding a content with one view:

```xml
<Accordion name="recipeParts" at="5,1" span="8" rows="14" orientation="horizontal">
  <Section title="@overviewSection">
    <Content name="recipeOverviewContent" at="1,1" span="8" rows="14">
      <View name="RecipeOverview" onsaved="recipeSaved" />
    </Content>
  </Section>
  <Section title="@timerSection">
    <Button name="startTimerButton" at="1,2" span="3" caption="@startTimerCaption" onclick="startTimer" />
  </Section>
</Accordion>
```

An application has one desktop window, on every stack. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A desktop window of its own opens only where the design says so: a target marked `newWindow="true"` starts one, and a dialog may get one too (the web stacks show it on top of the window that opened it; on C# dialogs come with Clarity.UI).

### Settings

A window's settings are attributes of its root element; its data are `Data` elements, the first things inside it.

| Setting | Meaning |
|---|---|
| `title="@key"` | The window's title, in its title bar; a key may hold `{app.version}`, as `mainTitle "Todo v{app.version}"`. Without one, the window shows the app's title. |
| `width="320" height="460"` | The size its desktop window opens at, in pixels: 200 to 3840 wide and 150 to 2160 high. Without it a window opens at 1100 x 760. A desktop window takes the size of the window it shows first; a window opened in place keeps the desktop window's size. The designer shows a window at its size. Its parts end by the last row its height holds: its height less 12 pixels of padding above and below and the frame's bars (a menu bar 30, a toolbar 38, a ribbon 30 and 92 more while open, a footer 28 or a row and 8 with parts; a dialog has only its own), in rows of the project's row height with gaps of 8. Every design edit (the designer's and the agents' tools) cuts a part to end by that row, a slide-out to that many rows, and a view's parts to the rows the window holds from its content area's top; the window keeps its size. A part placed wholly below stays, and the checker warns that the window scrolls, with the height that would hold it. |
| `style="name"` | A style of the project's theme for the window itself: its background fills the whole window and its text color is its parts' text, as a splash screen's deep color. Every theme the application offers must have it. |
| `<Data name="customer" type="Customer" />` | One record the window works on. |
| `<Data name="orders" type="list&lt;Order&gt;" />` | A list of records the window works on. |
| `type="Record"`, `type="list&lt;Record&gt;"` | A [record](#records) the window holds, new when it opens, or a list of them, empty when it opens; code fills them, nothing stores them. |
| `<Data name="shown" type="decimal" />` | A plain value the window holds: `type="decimal"`, `type="Operation"`, `type="list&lt;decimal&gt;"`. The type is `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`, an enum, or a list of text, integers, decimals or dates, which starts empty. It is no record and is never stored; it lives as long as the window. |
| `<Data name="fresh" type="bool" default="true" />` | A plain value that starts at its `default`. Without a default it starts empty: 0, empty text, false, no date, or the enum's first value. |
| `columns="N"` | The number of grid columns, 1 to 96. The default is 12. |
| `columns="56" cell="10"` | The page's own exact grid, for a designed page such as a splash: rows of `cell` pixels (8 to 64), cells without gaps or padding, rows that keep their height. `columns="56" cell="10"` on a 560 px window is square cells of 10 pixels. |
| `onopen="handler"` | Runs when the window has opened and its data is loaded. |
| `onclose="handler"` | Runs when the window is about to close. |
| `onreturn="handler"` | Runs when a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record. |

The window's data is what its parts bind to and what its handlers work on. When a window is opened with `open CustomerForm with customer = ...`, the data is filled by name. Data that is not filled starts as a new, empty record, for a list as all records of the entity, and for a plain value at its default. A plain value cannot be read again with `reload`.

### The grid

Every part sits on the grid of the window or of the container it is in:

- `at="col,row"` is the top-left cell, counted from 1. Every part needs one.
- `span="n"` is the number of columns it covers. The default comes from the part's kind.
- `rows="n"` is the number of rows it covers. The default also comes from the part's kind.

Rows have a fixed height in the designer. In a running application the columns share the width of the window, and rows that hold something that grows (a table, a list, an image, a canvas, a memo, a custom part declared with `grows`, or a container with one of these inside) share the extra height. Everything else keeps its height, also in a row that grows: it stands at the top of its rows, as high as they are at their least.

When no row of a grid grows and the grid is higher than its rows, `bottom="true"` on a part sticks it to the bottom: the rows from the first row of a part with `bottom` down move down by the height to spare, keeping their heights. Buttons under a form in a pane that a list beside it makes taller sit at the bottom of the pane: `<Button name="save" at="3,11" span="2" caption="@saveCaption" bottom="true" />`. Where a row grows, `bottom` changes nothing.

The checker reports parts that start outside the grid, run past its last column, or overlap another part in the same container.

## 6. Parts

In a window's or view's file a part is an element: its kind as the element's name, its properties as attributes, and the parts it holds inside it.

```xml
<Textbox name="email" at="4,2" span="6" bind="customer.email" placeholder="@emailPlaceholder" onchange="emailChanged" />
```

The tables below name the properties in Clarity's own words, as the checker's messages say them (`bind customer.email`, `on change: emailChanged`). In a `.cxaml` file they are written this way:

| Clarity's words | In a `.cxaml` file |
|---|---|
| the kind, `textbox`, `treetable` | the element, `<Textbox>`, `<TreeTable>` |
| the caption, `button @saveCaption` | `caption="@saveCaption"` |
| `at 4,2`, `span 6`, `rows 3` | `at="4,2"`, `span="6"`, `rows="3"` |
| a word alone, `default`, `bottom`, `sortable` | `default="true"`, `bottom="true"`, `sortable="true"` |
| a word and a value, `bind customer.email`, `columns number, total` | `bind="customer.email"`, `columns="number, total"` |
| `on change: emailChanged`, `on click: digit(7)` | `onchange="emailChanged"`, `onclick="digit(7)"` |
| `on click -> CustomerBrowse`, `in new window` | `opens="CustomerBrowse"`, `newWindow="true"` |
| `key "Ctrl+S"` (a shortcut) | `key="Ctrl+S"` |
| `children children` | `childList="children"` |
| `key id parent parentId` (a tree's rows) | `rowKey="id" parentKey="parentId"` |
| `width 420` of a slide-out | `slideWidth="420"` |
| `horizontal` of an accordion | `orientation="horizontal"` |
| `reorder by position` | `reorder="position"` |
| `view RecipeEditor` in a content | `<View name="RecipeEditor" />` inside the `<Content>` |
| a value with several fields, such as a canvas's `units 160 x 100` or its tells | JSON, as the designer writes it: `units="{&quot;height&quot;:100,&quot;width&quot;:160}"` |

A part writes only what differs from its kind's defaults. The designer writes the attributes in a fixed order, which is the order the tables below use.

In the samples a window shows its parts in a view, as every window does (see [Views](#views)); a dialog holds its parts itself.

### Properties every part has

| Property | Meaning |
|---|---|
| `"caption"` | The text it shows, right after the kind, for the kinds that have one. |
| `name n` | The part's name, unique in its window (sidebars, footer, menu, toolbar and ribbon included). Handlers refer to the part by it. The designer names new parts after their purpose (`vatNumberLabel`, `ordersTable`, `addLine`), or `button1` until it knows it; templates name inputs after their field. |
| `at col,row`, `span n`, `rows n` | Place and size on the grid; see [the grid](#the-grid). |
| `bottom` | Sticks to the bottom of its grid when no row grows; see [the grid](#the-grid). |
| `tooltip "text"` | Shown when the pointer rests on the part. |
| `style name` | A style of the project's theme: its colors, and the size, font and alignment of its text. See [Themes](#themes). |
| `on event: handler` | Runs `handler` in the window's code behind when `event` happens. Each event is wired at most once per part. |
| `on event: handler(values)` | Runs `handler` with these values, one per parameter of the handler: `on click: digit(7)`, `on click: operation(Add)`. A value is a number, text in quotes, `true`, `false` or a value of the parameter's enum. |

### The catalog

**Text**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `label` | nothing | `for input` | none |
| `help` | nothing | `for input` | none |
| `heading` | a field, or nothing: `bind chapter.title` shows its value in the heading font, else its caption | none | none |
| `icon` | nothing; its first word is the icon it shows: `icon "triangle-alert"` | none | none |
| `badge` | a text, integer, decimal, money or enum value, or its caption | none | none |
| `link` | a url, email or text value, or its caption with `to "https://..."` or `on click` | `to`, `taborder` | `click` |
| `separator` | nothing: a line, level or `vertical` | `vertical` | none |
| `progress` | an integer or decimal value, from 0 to `max` (100 without it) | `max 250` | none |
| `busy` | nothing: a running indicator with its caption, shown and hidden by code with `visible` | none | none |
| `text` | a field of any type: `bind customer.name` | none | none |
| `markdown` | a key whose value is a Markdown file per language, `@gameRules`; a file of the resources, `source "license.md"`; or a text, `bind notes` | `source "file"` | none |

**Input**

| Part | Binds to fields of type | Own properties | Events |
|---|---|---|---|
| `textbox` | text, email, url | `placeholder`, `copy`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `memo` | text (several lines) | `placeholder`, `code csharp`, `markdown`, `copy`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `password` | text (hidden while typed; an eye beside it shows it or hides it again, and a button copies it) | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `signin` | nothing: it signs in with the project's `signin` block, on the app's splash | none: the block's `shows` and `remember ... when asked` say what it offers; see [Signing in](#signing-in) | none |
| `signinField`, `signinButton`, `signinRemember`, `signinLanguage`, `signinOther`, `signinRefusal` | nothing: the parts of a sign-in view | `input`, `strategy`, `shows`; see [Signing in](#signing-in) | none |
| `number` | integer, decimal, money, id | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `checkbox` | bool | `readonly`, `taborder` | `change` |
| `switch` | bool | `readonly`, `taborder` | `change` |
| `dropdown` | an enum, `ref Entity`; text with `typed` | `typed`, `search`, `show name, city`, `placeholder`, `readonly`, `taborder` | `change` |
| `radio` | an enum, as one button per value | `horizontal` or `vertical`, `readonly`, `taborder` | `change` |
| `date` | date, datetime | `placeholder`, `readonly`, `taborder` | `change` |
| `color` | color | `placeholder`, `readonly`, `taborder` | `change` |
| `file` | file | `readonly`, `taborder` | `change` |
| `folder` | a folder setting (a path typed, or chosen with the system's folder picker) | `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `tags` | a list of values (`list<text>`, `list<Genre>`) | `placeholder`, `readonly`, `taborder` | `change` |
| `slider` | integer, decimal, money | `min`, `max`, `step`, `readonly`, `taborder` | `change` |
| `spinbox` | integer, decimal, money (a number with up and down arrows) | `min`, `max`, `step`, `placeholder`, `readonly`, `taborder` | `change`, `enter`, `leave` |
| `button` | nothing | `taborder`, `key`, `default`, `cancel`, `icon "save"`, `split` | `click` |
| `toggle` | bool, or its own `checked` when unbound | `readonly`, `taborder`, `icon "bold"` | `change` |

A slider and a spin box move a number between its lowest value (`min`) and its highest (`max`), a `step` at a time (1 without it). Without `min` or `max` they take the field's own `min` and `max`; a slider without either runs from 0 to 100, and a spin box without them goes on without end. The arrow keys move a step, Page Up and Page Down ten; on a slider Home and End go to the ends. A slider takes the nearest step where it is let go, and tells its `change` then; a number typed into a spin box is kept within its range. The checker wants `min` below `max`, a `step` above 0, and a whole step for an integer field. Code reads and sets a spin box's range as whole numbers, `levelBox.min` and `levelBox.max`: as its line writes them (0 and 2147483647 where it writes none) until a handler sets them, as a dial that goes up to a domain's newest version does with `levelBox.max = top`.

```clarity
slider name discountSlider at 1,1 span 6 bind line.discount min 0 max 50 step 0.5 on change: discountMoved
spinbox name quantityBox at 7,1 span 2 bind line.quantity
```

A color input shows a swatch and the color as `#rrggbb`. The swatch opens a palette of 60 colors, the same on every stack; a color can also be typed, with or without `#`, in three or six digits (`f0a` is `#ff00aa`). Text that is no color stays as typed and is marked, and the field keeps its color. Tables show a color column with its swatch and edit it with the same input.

**Containers**

| Part | Holds | Own properties | Events |
|---|---|---|---|
| `form` | parts, on its own grid; binds to one record: `bind customer` | `foldable`, `foldable folded`, `labels 3` | `submit` |
| `panel` | parts, on its own grid, in a frame with the caption | `foldable`, `foldable folded` | none |
| `tabs` | `tab @key` blocks, each with parts or `view X`; code reads and sets the open tab as `part.tab`, counted from 1 | none | `change` |
| `steps` | `step @key` blocks, each with parts or `view X`: a wizard | none | `next`, `finish` |
| `popover` | parts on its own grid, opened next to the part in `for` | none | none |
| `slideout` | parts on its own grid, sliding in over the window from its right, left, top or bottom edge | `side left` (`top`, `bottom`), `width 420` (`height 300` at the top or bottom), `resizable`, `min 300`, `max 900`, `pinnable` | `open`, `close` |
| `buttons` | buttons and toggles joined in one row, on its own grid | none | none |
| `accordion` | `section @key` blocks, each with parts or `view X` and an optional `color`; one open at a time | `horizontal` | `change` |
| `scroll` | parts, on its own grid; parts below its rows scroll into view | none | none |
| `splitter` | exactly two `pane` blocks, each with parts; `pane span n` starts the first at n columns | `horizontal` or `vertical` | none |

A panel or form with `foldable` has a header the person clicks to fold it to that header and unfold it again; `foldable folded` starts it folded. Code reads and sets whether it is folded as `part.folded`, so a handler can open the section a message is about. In the designer it stays open, to edit what it holds. On the web stacks the renderer folds it; on C# it comes with Clarity.UI's parts.

**Data**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `table` | a list: `bind orders` | `columns field [width], ...`; `edit field [as switch], ...`; `resizable`; `sortable`; `add`; `remove` [`confirm`]; `order by field [desc]`; `reorder by field`; `page 50`; `striped`; `frame` | `select`, `open`, `change`, `reorder`, `add`, `remove` |
| `list` | a list: `bind orders` | `show field, field, ...`; `cards`: each row a small card, the first field of `show` its title and the others lines below it, in the theme's `part card` and `part card_line` looks; `reorder by field`; `striped`; `frame` | `select`, `open`, `reorder` |
| `tree` | a list: `bind folders children children` | `children field`, or `key field parent field`; `show field, ...`; `reorder by field`; `striped`; `frame` | `select`, `open`, `reorder` |
| `treetable` | a list: `bind folders children children` | `children field`, or `key field parent field`; `columns field [width], ...`; `resizable`; `add`; `remove` [`confirm`]; `reorder by field`; `striped` | `select`, `open`, `reorder`, `add`, `remove` |
| `chart` | a list: `chart bar ... bind orders by status count`; or a list of numbers, drawn as they are: `chart line ... bind readings` | `bar`, `line` or `pie` first; `by field`; `count`, `sum field` or `average field`; for a list of numbers `points n` (its last n) and `max n` (the top of the axis) | |
| `trend` | an integer, decimal, money or duration: `bind cpu` | `points n`, the values it keeps, 60 without it; `max n`, the top of the axis; `every n`, seconds between values, 1 without it; see [Live graphs](#live-graphs) | none |

**Graphics**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `image` | an `image` field, or nothing | `source "file"` for a fixed picture of the resources folder | `click` |
| `display` | an integer, decimal or money: `bind shown` | `digits n`, 1 to 24, 12 without it | none |
| `canvas` | nothing; an `image` field in `mode paint`, a `text` field in `mode vector` | `units W x H`, 1 to 10000 each, 100 x 100 without it; `zoom`; `grid N` and `snap`; `background color`; `dots N color`; `surface svg`, `surface bitmap` or `surface gpu`; `mode objects list at x, y`, `mode paint` or `mode vector`, with `tools`, `brush N` and `ink color` | `draw`, `click`, `drag`, `drop`, `bend`, `pointer`, `key`, `change` |

**Media**

| Part | Binds to | Own properties | Events |
|---|---|---|---|
| `video` | nothing | `source "file"` for a video of the resources folder; `volume 0` to `100`; `repeat`; see [Video](#video) | `end`, `fail` |
| `sound` | nothing; it has no place in the window: no `at`, `span` or `rows` | `source "file"` for a sound of the resources folder; `volume 0` to `100`, 100 without it; `repeat`; see [Sound](#sound) | `end`, `fail` |
| `timer` | nothing; it has no place in the window: no `at`, `span` or `rows` | `every n`, seconds between its ticks, 0.1 to 86400; see [Live graphs](#live-graphs) | `tick` |

`select` fires when another row is chosen, and `open` on a double-click or Enter. Rows of a part with `on open` show a hand under the pointer and say how to open them (Clarity's word `openRow`), so people see they lead somewhere. `reorder` fires when the person moved a row of a part that reorders; see [Reordering](#reordering). `enter` and `leave` fire when the focus moves into and out of an input.

`change` fires when the person changes an input's value to one that differs, and only then, on every stack:

- Typed text is taken when the input is left or Enter is pressed; typing alone is no change, and neither is leaving without typing or typing the same text again. In a `memo`, Enter is a new line and the text is taken when it is left.
- A number reads in the person's format. Text that is no number stays as typed and is marked, and the value stays; a cleared number is zero.
- A checkbox, switch, dropdown, radio group or date changes when the person ticks or picks another value. An enum offers no empty choice; a `ref` offers one first, for no record.
- A `typed` dropdown takes any text, and offers the values its field already has as it is typed: `dropdown name cityBox at 1,1 span 4 bind customer.city typed`. A value it offers once, sorted ignoring case; typing filters them. Picking one or leaving the box with new text is the change.
- A `search` dropdown picks a reference among many records: typing filters them, a record staying when every word typed is in its text or a field it shows. `show` names the fields each record shows beside its text: `dropdown name customerBox at 1,1 span 6 bind order.customer search show name, city`. Emptying the box picks no record. Every stack offers and filters alike (`conformance/dropdown-options.json`).
- What a handler sets, and a record read again, is never a change.
- Enter in an input of a `form` takes the input first, then runs the form's `on submit`.

### Editing in a table

`edit` names the columns whose cells are inputs, so a row can be changed where it is shown:

    table name grid at 1,2 span 12 rows 8 bind tasks columns title, done edit done on change: taskChanged

    handler taskChanged
        save grid.selected

- A `bool` column holds a checkbox, or a switch when it is written `as switch`: `edit title, done as switch`. An enum column is a dropdown of its values, and a `ref` column a dropdown of the records it can point at, as a dropdown in a form shows them. Dates are picked with a date picker, as in a date input. A moment (`datetime`) is a day from the date picker and a time typed on a 24-hour clock (`14:30`), in a cell and in a date input alike. Text and numbers are edited in place, like a text box or number input.
- An edited column must be one of the `columns`, of one of these types: text, `email`, `url`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an enum or a `ref`. Pictures and durations are not edited in a cell.
- Only a value that differs is an edit: a checkbox or dropdown when it changes, a text or number cell when it is left or Enter is pressed. An edit changes the row on screen. Then the row is selected and `on change` runs, so `grid.selected` is the changed record. Saving it is up to the handler, as with a form: `save grid.selected`. When the record breaks a rule, `save` refuses it as anywhere else.
- A table that edits but has no `on change` is a warning: its edits are lost when the data is read again.

### Columns of a table

A column may have a width, and a table may let the person resize and sort it:

    table name albums at 1,2 span 12 rows 8 bind albums columns title 240, artist, year 60 resizable sortable order by year desc

- A width in pixels follows a field: `title 240`. A column without one shares the width that is left. Widths are 20 to 2000 pixels. On the design canvas, dragging the right edge of a header sets the width, and a double-click there takes it away.
- `link chapter: toChapter` makes a column's cells links, in the theme's accent and underlined, with a hand under the pointer: a click on one chooses its row and runs the handler, which finds the row as the table's `selected` (`placeTable.selected.chapter`). Several columns are written `link chapter: toChapter, layer: toLayer`. A link column is one the table shows and does not edit.
- `resizable`: the person drags the header edges to resize the columns. The widths are remembered for them, per window and table, and a double-click on an edge gives the designed widths back.
- `sortable`: a click on a header sorts the rows by that column, a second click the other way. Text sorts ignoring case, numbers as numbers, an enum or `ref` by the label it shows, and an empty value comes last either way.
- `order by field [desc]`: the order the rows show in when the window opens, as in queries. The field must be one of the `columns`.
- `page 50`: the rows show 50 at a time (1 to 1000), with a bar below the table to go to the first, previous, next and last page and the rows it shows, as `51-100 / 230`. The rows are ordered first, then paged. The table keeps its page when its rows come again, goes back to the first page when a header click sorts it, and shows the page of a row that code selects. The bar's buttons say Clarity's words `firstPage`, `previousPage`, `nextPage` and `lastPage`. Every renderer pages alike (`conformance/table-pages.json`).

### Charts

A `chart` draws a list the window holds, its rows grouped by a field and each group given a number:

```xml
<!-- shop/design/Dashboard.cxaml -->
<Window class="shop.Dashboard">
  <Content name="main" at="1,1" span="12" rows="12">
    <View name="DashboardView" />
  </Content>
</Window>
```

```xml
<!-- shop/design/DashboardView.cxaml -->
<View class="shop.DashboardView">
  <Data name="orders" type="list&lt;Order&gt;" />
  <Chart name="byStatus" at="1,1" span="6" rows="6" bind="orders" by="status" />
  <Chart name="perDay" at="7,1" span="6" rows="6" bind="orders" chart="line" by="placed" measure="{&quot;field&quot;:&quot;total&quot;,&quot;kind&quot;:&quot;sum&quot;}" />
  <Chart name="share" at="1,7" span="4" rows="6" bind="orders" chart="pie" by="customer" measure="{&quot;field&quot;:&quot;total&quot;,&quot;kind&quot;:&quot;average&quot;}" />
</View>
```

- The word after `chart` is what it draws: `bar`, `line` or `pie`.
- `by field` names the field whose values become the bars, points or slices; rows with no value there are left out. `count` gives each group its number of rows, `sum field` the sum of a number field, `average field` its average over the rows that have one.
- The groups come in the order of their values: numbers and dates low to high, text ignoring case, an enum's values as it declares them, no before yes, a reference by the text of the record it names. Each shows as people read it: an enum by its label, a date in their format.
- A bar or line chart has an axis in steps of 1, 2, 2.5 or 5 times a power of ten that always takes in 0. A pie's slices take the colors of the palette's fourth row, in order; a group of 0 or less has no slice. Each bar, point and slice tells its group and number when pointed at.
- The chart draws again when its rows change. Every renderer groups and draws alike (`conformance/charts.json`).
- Bound to a list of numbers (`<Data name="readings" type="list&lt;decimal&gt;" />`, or such a field of a record), a bar or line chart draws the numbers as they are, in order, without groups: `chart line name history at 1,1 span 6 rows 4 bind readings points 30 max 100`. `points n` shows the list's last n numbers, `max n` fixes the top of the axis. A line of more than 30 points has no dot at each. See [Live graphs](#live-graphs).

### Live graphs

A window shows numbers as they change with a `timer`, which runs a handler every so many seconds, and a `trend` or a chart of a list of numbers, which draw them (DESIGN 5.26):

```xml
<!-- monitor/design/Monitor.cxaml -->
<Window class="monitor.Monitor">
  <Content name="main" at="1,1" span="12" rows="8">
    <View name="MonitorView" />
  </Content>
</Window>
```

```xml
<!-- monitor/design/MonitorView.cxaml -->
<View class="monitor.MonitorView">
  <Data name="cpu" type="decimal" />
  <Data name="readings" type="list&lt;decimal&gt;" />
  <Timer name="clock" at="1,1" span="1" every="1" ontick="sampled" />
  <Trend name="cpuTrend" at="1,1" span="6" rows="4" bind="cpu" max="100" points="60" />
  <Chart name="history" at="1,5" span="6" rows="4" bind="readings" chart="line" max="100" points="30" none="measure" />
</View>
```

```clarity
code MonitorView
    handler sampled
        cpu = load()
        readings = readings + [cpu]
```

- **A timer** has no place in the window; the designer keeps it in the tray under the window, with the sounds. While its window is open it runs its `on tick` handler every `every n` seconds (0.1 to 86400). A tick that comes while a handler still runs is left out, not saved up. `clock.enabled = false` stops it until code sets it back to true.
- **A trend** follows one number: every `every n` seconds (1 without it) it takes the value its number has, and keeps the last `points n` (60 without it), the newest at the right. It does not wait for code: what code last set is what it takes. `max n` fixes the top of its axis, which otherwise takes in its largest value.
- **A list of numbers** grows with `+`: `readings + [cpu]` is a new list with the item after the others; both sides are lists of the same kind of item. A chart of the list draws its last `points n` numbers.

### Graphs

A `graph` shows records as nodes joined by edges: the records of the list it binds are its nodes, the records of the list `edges` names are its edges. Both are records of any kind the project declares, with whatever fields they need; the graph only has to know which fields play which part:

```clarity
entity Thing
    name: text
    kind: text
    owner: text

entity Link
    source: integer
    target: integer
    field: text
    since: integer
```

```xml
<!-- ideas/design/MapView.cxaml -->
<View class="ideas.MapView">
  <Data name="things" type="list&lt;Thing&gt;" />
  <Data name="links" type="list&lt;Link&gt;" />
  <Graph name="map" at="1,1" span="12" rows="10" bind="things" edges="links" nodeLabel="name" nodeKind="kind" edgeLabel="field" layout="force" nodeLooks="{&quot;person&quot;:{&quot;shape&quot;:&quot;box&quot;,&quot;color&quot;:&quot;accent&quot;}}" onselect="picked" onopen="opened" />
</View>
```

- **A node** is named by its key, `nodeKey` (`id` without it); `nodeLabel` is the field it shows (its key without it), `nodeKind` the field whose value chooses its look.
- **An edge** joins the node whose key its `edgeFrom` field holds (`source` without it) to the node whose key its `edgeTo` field holds (`target` without it). A key is matched by its value, and a reference field may hold it, so a list can be its own edges: `bind="folders" edges="folders" edgeFrom="id" edgeTo="parent"` draws folders under their parents. An edge whose ends are not both shown is left out. `edgeLabel` and `edgeKind` are an edge's label and kind.
- **Every other field** is the record's own. Pointing at a node tells its label and its fields, the ones `show` names or else every field that plays no part; pointing at an edge tells its label, the nodes it joins and its fields.
- **`layout`** places the nodes: `force` (the default) pushes them apart and pulls them together along their edges, `layered` puts them in layers along the edges' direction, `radial` in rings around the picked node, `circle` on one circle. Each places the same graph the same way every time.
- **`nodeLooks`** gives the nodes of a kind, by the kind's value, a `shape` (`circle`, `square`, `diamond`, or `box` with the label inside), a `color` (a color of the theme or written out) and a `size` across in pixels; a kind without a look is a circle in the next color of the palette's fourth row. **`edgeLooks`** gives an edge kind a `line` (`solid`, `dashed`, `dotted`), a `color` and `arrow` (an arrow at its target unless `false`). The kind `""` is the look of nodes or edges without a kind.
- **`limit n`** is the most nodes it shows, the first ones of the list (500 without it, at most 5000); a graph cut short says so in its corner (Clarity's word `graphCut`).
- The person zooms with the buttons in its corner (Clarity's words `zoomOut`, `zoomWhole`, `zoomIn`), with Ctrl and the wheel, and with + and -; 0 shows it whole again. The wheel or a drag of the background pans.
- A drag of a node moves it, and it stays where it was left: as the graph changes, and the next time the window opens, kept for the person by the window's and the graph's name (as a table keeps its column widths). A node is kept by its record (its key, or in a graph of lists its entity and id), not by its place in the list. Picking, hops and the legend move no node: the whole graph is laid out, and they only keep some of it from showing.
- A click on a node picks it and runs `on select` when it is another one; every click runs `on click`, or opens what `opens` names, as a button's: `opens="nodeDetails"` slides out a slide-out with the picked record's details. A double click or Enter runs `on open`. Code reads the picked record as `map.selected`, as a table's. The graph draws again when its lists change.

#### A graph of the data model

Instead of a list it binds and its edges, a graph can name lists of records of different entities, `lists`: their records are its nodes, and the data model gives the rest. A record's kind is its entity, its label its entity's first text field (as a reference shows a record), and each reference field to an entity the graph shows is an edge, labelled with the field. No code is needed: a window's lists hold every record when it opens.

```xml
<View class="shop.CustomerMapView">
  <Data name="customers" type="list&lt;Customer&gt;" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Data name="kindPicked" type="text" />
  <Data name="idPicked" type="integer" />
  <Graph name="map" at="1,1" span="12" rows="10" lists="customers, orders" hops="2" legend="true" pickedKind="kindPicked" pickedId="idPicked" onopen="opened" />
</View>
```

- A record in two lists is one node; records of two entities with the same id are two. A reference to an entity the graph does not show is no edge.
- A graph of lists binds nothing and names no edges or node fields. Its looks, layout, `show` (fields of any of its entities) and `limit` are a graph's as above.
- **`pickedKind`** and **`pickedId`** name text and whole-number data: picking a node writes its entity and id there, and code that sets them picks that node. The handlers of `on select` and `on open` read them: `if kindPicked == "Order"`.

`hops n` (1 to 10), on any graph, shows only the nodes at most that many edges from the picked node, either way along them, with a control in its corner to show fewer or more (Clarity's words `graphHops`, `graphFewerHops`, `graphMoreHops`); with nothing picked it shows all of it. `legend` shows each kind as a chip in its corner, in its color; a click hides or shows that kind (`graphKind`). The picked node always shows.

### Trees

A `tree` and a `treetable` show a list the window holds as a tree. The entity has one reference to its own kind, a row's parent, and a list field of its own kind that stands for the rows under it:

```clarity
entity Folder
    name: text(80), required
    size: integer
    parent: ref Folder
    children: list<Folder>
```

```xml
<!-- files/design/Files.cxaml -->
<Window class="files.Files" onopen="loadFolders">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="FilesView" />
  </Content>
</Window>
```

```xml
<!-- files/design/FilesView.cxaml -->
<View class="files.FilesView">
  <Data name="folders" type="list&lt;Folder&gt;" />
  <Tree name="folderTree" at="1,1" span="4" rows="10" bind="folders" childList="children" />
  <TreeTable name="folderTable" at="5,1" span="8" rows="10" bind="folders" columns="name, size" widths="{&quot;name&quot;:200}" childList="children" />
</View>
```

```clarity
code Files
    handler loadFolders
        folders = find f in Folder order by f.name
```

- A row whose parent is in the list shows under it; any other row shows at the top. Each level keeps the order of the list. A loop of parents is broken at its row that comes first in the list.
- A row with rows under it has a toggle that shows or hides them; all start closed. Its words are Clarity's `expandRow` and `collapseRow`.
- A tree shows each row's text, or the fields of `show`. A tree table shows `columns` under their headers, the first one indented.
- `folderTree.selected` is the record picked, as for a table, and selecting a row in code (`folderTree.selected = folder`) opens the rows above it. Every renderer arranges the rows alike (`conformance/trees.json`).

A tree can also be arranged **by key**: `key` names the field that names a row and `parent` the field that holds the key of the row it shows under, text or whole numbers of the same type. The rows need no reference to their own kind, so code builds one tree of different things as a list of records (see [Records](#records)); a stored record's `id` names it too.

```clarity
tree name nav at 1,1 span 4 rows 20 bind nodes key key parent parent show label
treetable name grid at 5,1 span 8 rows 20 bind nodes key key parent parent columns label 200, kind
```

- A row shows under the first row whose key its parent field holds; any other row shows at the top. Empty text names no row, so a row whose parent is empty is at the top. Rows keep the order of the list, and a loop of parents is broken as above.
- A tree has `children` or `key` and `parent`, not both. In the designer they are **Key** and **Parent**, offered when the list's rows have text or integer fields.
- Sorting changes only how the rows show. A row keeps its place in the data, so `grid.selected` and every event name the record the person sees, and `save grid.selected` saves it.

### Adding and removing rows

A table (and a tree table) adds and removes its own rows when its line says `add` and `remove`: an Add button stands above its header at the right, and every row has a trash button in a last column of its own. The Add button runs `on add`; a trash button picks its row and runs `on remove`, which reads the row as `table.selected`. Insert and Delete do the same from the keys, and a read-only or disabled table runs neither. The handlers do the adding and deleting, so each needs its handler.

```xml
<!-- modelling/design/ClassEditor.cxaml -->
<View class="modelling.ClassEditor">
  <Data name="editedClass" type="ModelClass" />
  <Data name="properties" type="list&lt;Property&gt;" />
  <Table name="propertyTable" at="1,1" span="6" rows="6" bind="properties" columns="name, type" edit="name, type" add="true" remove="true" onadd="propertyAdded" onremove="propertyRemoved" onchange="propertyChanged" />
</View>
```

```clarity
code ClassEditor
    handler propertyAdded
        let p = new Property
        p.owner = editedClass
        save p
        properties = find x in Property where x.owner == editedClass

    handler propertyRemoved
        delete propertyTable.selected
        properties = find x in Property where x.owner == editedClass
```

With `remove confirm` the trash button (and Delete) asks first, in Clarity's dialog: "Remove {row}?" with the row's first column, and Remove and Keep; `on remove` runs on Remove only. For a question of its own (a dialog page with more in it, say) the `on remove` handler asks in code instead, with `ask` or `open`.

The button says Clarity's word `addRow` ("Add") with the plus icon, and the trash button's tooltip is `removeRow` ("Remove"); the question is `removeRowAsk` and its other button `keepRow`. Each language file gives them its own words, as `addRow "Toevoegen"` and `removeRow "Verwijderen"`.

### Reordering

A `list`, `table`, `tree` or `treetable` lets the person put its rows in their own order when its line says `reorder by` and an integer field of the listed entity, which keeps the order (DESIGN 5.30). It is an option: without it the rows stay where the data has them. In the designer it is the Reorder by the person checkbox, with the field it is kept in.

```clarity
entity Task
    title: text, label @title
    position: integer
```

```xml
<!-- todo/design/Tasks.cxaml -->
<Window class="todo.Tasks" onopen="loadTasks">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="TasksView" />
  </Content>
</Window>
```

```xml
<!-- todo/design/TasksView.cxaml -->
<View class="todo.TasksView">
  <Data name="tasks" type="list&lt;Task&gt;" />
  <List name="taskList" at="1,1" span="4" rows="10" bind="tasks" reorder="position" show="title" onreorder="tasksMoved" />
</View>
```

```clarity
code Tasks
    handler loadTasks
        tasks = find t in Task order by t.position

code TasksView
    handler tasksMoved
        for t in tasks
            save t
```

- **Moving a row.** The person drags a row with the pointer, and a line shows where it lands; or chooses a row and presses Alt+Up or Alt+Down. In a tree a drop on the middle of a row puts the dragged row under it, as its last row; Alt+Left puts a row after its parent, a level up, and Alt+Right puts it under the row before it. A row takes the rows under it along. Every renderer moves alike (`conformance/reorder.json`).
- **What it changes.** The list in the window's data takes the new order, and every row gets its number in the field: its place at its level, counted from 1, or from the number `from` gives (`reorder by chapter from 0`, for a book whose prologue is chapter 0). In a tree the moved row also gets its new parent. A move that would change no number or parent is no move: nothing happens.
- **`on reorder`** runs after the move, with the moved row chosen (`taskList.selected`). Saving is up to the handler, as with a table that edits; a part that reorders without `on reorder` is a warning, since its order is lost when the data is read again. Load the rows in their order: `order by t.position`.
- **Undo.** A move and what its `on reorder` saved are one edit, which Ctrl+Z takes back, unless the line says `no undo`; see [Undo](#undo).
- **A table** that reorders has no `sortable` and no `order by`: its rows keep the person's order. With `page`, a row moves on its page by dragging, and to the page before or after with Alt and the arrows.
- **Not in the designer**: there the rows are samples, and do not move.
- **Between parts.** A list, table or tree takes rows dragged onto it from another one of the window with `on drop from fragmentList: fileFragment`, one line per part it takes rows from. The person drags a row out of `fragmentList` and lets it go on a row of this part: the dragged row is chosen in `fragmentList` and the row it landed on in this part (beside the rows, its chosen row stays), then the handler runs and reads both as `fragmentList.selected` and `chapterList.selected`, to link them. The rows themselves stay where they are. A memo takes rows too (`memo ... on drop from fragmentList: fragmentDropped`): the caret goes where the row was let go in its text, then the handler runs, so `manuscriptBox.insert(fragmentList.selected.text)` puts the fragment in there.

### Binding

`bind` connects a part to the window's data:

- An input or `text` binds to one field of a single record: `bind customer.name`, or to a plain value by its name: `bind shown`. The type must be one the part accepts; a checkbox needs a `bool`. A plain `decimal` shows the digits it has.
- A `table` or `list` binds to a list: `bind orders`. Its `columns` or `show` name fields of the listed entity.
- A `form` binds to a single record: `bind customer`.

What the user types goes into the data, and handlers read and change the data by name. A part that cannot be bound, such as a label, may not have `bind`. An input that is not bound yet is allowed while designing.

### Labels, help texts and forms

A label, its input and the input's help text are three separate parts. `for` ties a label or a help text to an input by the input's name:

```clarity
form name details at 1,1 span 12 rows 3 bind customer
    label @nameLabel name nameLabel at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name placeholder @fullName
    help name nameHelp at 10,1 span 3 for name
    label @kindLabel name kindLabel at 1,2 span 3 for kind
    radio name kind at 4,2 span 6 bind customer.kind
```

- **A label** shows its caption. Clicking it focuses its input. When the input is bound to a `required` field, the label shows a marker. A label is never bound to data; to show a value, use `text`.
- **A help text** shows its caption, or, when it has none, the `help` of the field its input is bound to. When a handler sets the input's `error`, the help text shows that message instead, and the input is marked.
- **`for`** must name an input in the same window.
- **A form** binds to one record and holds the inputs of that record, nested in panels and tabs if needed. Every input inside it binds to that record, and a form cannot contain another form. Pressing Enter in one of its inputs fires `submit`; when `submit` is not wired, Enter clicks the window's default button. A form without a caption has no frame, so its grid lines up with the grid around it.

### Input and button properties

| Property | On | Meaning |
|---|---|---|
| `code csharp` | memo | Code: the text in the mono font, its comments, texts, numbers, keywords and type names in the theme's colors (`code_keyword`, `code_type`, `code_text`, `code_number`, `code_comment`); read only, it takes the theme's `code` part look, its own background (`code_background`) and a quieter border. The languages: clarity, csharp, rust, go, c, cpp, java, javascript, typescript, python, sql, json, xml, html, css, yaml, markdown, shell, and diff, whose added and removed lines take `code_added` and `code_removed`. |
| `copy` | textbox, memo | A button in its corner (a memo's top right) puts its text on the clipboard, read only too; Clarity's word `copyText` names it. |
| `placeholder @key` | textbox, memo, password, number, spinbox, folder, date, color, tags, dropdown | Shown in a muted color while the input is empty: nothing typed, no day, no color, no tag, nothing chosen. Tags without one say Clarity's word `addTag`. |
| `readonly` | inputs | Shows the value, which cannot be changed. Handlers can change this. |
| `no undo` | the parts the person edits: inputs, images, canvases, tables with `edit`, and lists, tables and trees that reorder | Ctrl+Z takes back nothing on this part, not even typing in a text input. Every other one has undo; see [Undo](#undo). |
| `taborder n` | inputs, buttons | Position in the keyboard order, from 1. Parts without one follow in reading order. |
| `key "Ctrl+S"` | buttons | The key that clicks the button. |
| `default` | buttons | Enter clicks this button. One per window. |
| `cancel` | buttons | Escape clicks this button. One per window. |
| `horizontal`, `vertical` | radio, splitter | Side by side or stacked. |
| `frame` | list, table, tree, tree table | Its own border in a container (a tab page, a pane, a panel, a form, a section, a step, a scroll box), which otherwise frames it alone. On the window it always has one. |
| `striped` | list, table, tree, tree table | Every other row (the 2nd, the 4th, ...) has a faint stripe of the theme's text color, in a light theme and a dark one; a chosen row or the row under the pointer shows as without it. A list of `cards` has no stripes. Example: `table name orders at 1,1 span 12 rows 8 bind orders columns number, total striped`. |

A key is written as modifiers, then one key, joined by `+`: `"Ctrl+S"`, `"Ctrl+Shift+F2"`, `"Alt+Delete"`. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. The key is a letter, a digit, one of `+ - * / . , =`, `F1` to `F12`, or one of `Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`; `"Ctrl++"` is Ctrl and `+`. A key without Ctrl, Alt or Meta that is a character (`"7"`, `"+"`) is what is typed, from the keyboard or the number pad, on any keyboard layout; while the person types in a text input it is their typing and presses nothing.

### Clicking to open a window

A button, an image, a menu item, or a toolbar or ribbon button can open a window without code:

```clarity
button @allCustomersCaption name allCustomers at 1,12 span 3 on click -> CustomerBrowse
```

`on click -> Target` opens the window `Target`, in place, on top of the window the click is in. `on click -> Target with book, chapter` fills the opened window's data of those names from the data of the same names of the window the click is in, as `open Target with book = book` does in a handler; from a menu or toolbar of the app that is the window showing, and data that window does not have starts new. On a button the window must hold the data, of the same entity. `on click -> Target in new window` (in a `.cxaml` file `opens="Target" newWindow="true"`) opens it in a desktop window of its own instead: the one way, besides a `dialog`, that an application gets a second desktop window. Closing the last window there closes that desktop window, and the window that opened it gets its `on return`. In menus, toolbars and on buttons, `close` closes the current window, `quit` ends the application, and `undo` and `redo` take back or do again the last edit of the part that had the focus (see [Undo](#undo)). `back` and `forward` go through the places the person was in the window, as a browser does: each place is the chosen row of its lists, tables and trees and the open section, tab or step of its accordions, tabs and steps, noted once the person's click or key is done. Going back chooses those rows and opens those sections again, and their `select` and `change` handlers run as when the person did it; Alt+Left and Alt+Right do the same, unless the part with the focus takes those keys (a tree that reorders). `signin password` on a button of the app's splash signs in with that strategy, and `signout` signs out (see [Signing in](#signing-in)). In menus and toolbars, `language` lets the person choose the language the application shows (see [Translations](#translations)), and `theme` one of the themes it offers (see [Themes](#themes)). A click either opens a window this way or runs a handler (`on click: handler`), not both.

### Containers

A container has its own grid, as many columns wide as its `span`, with the column lines of the grid around it: its frame sits in the space between columns, so a label and an input inside a panel line up with those outside it. Its parts are indented below it, and their positions count from its own top-left cell:

```clarity
tabs at 1,5 span 12 rows 7
    tab @ordersTab
        table at 1,1 span 12 rows 5 bind orders columns number, total
    tab @notesTab
        text at 1,1 span 12 bind customer.notes
splitter at 1,1 span 12 rows 10 vertical
    pane
        list at 1,1 span 12 rows 4 bind customers show name
    pane
        panel @detailsCaption name details at 1,1 span 12 rows 4
            text at 1,1 span 6 bind customer.name
```

`tabs` need at least one `tab`, and a `splitter` has exactly two `pane` blocks. The person drags the divider between the panes. Side by side, the first pane can start at a width of its own, `pane span 4`: its grid has 4 of the splitter's columns and the second pane the rest, so a list on the left and a form on the right keep the window's column lines. Without it each pane has the splitter's columns and they start alike.

A form with `labels 3` has a label column: each input in it gets a label in the form's first 3 columns of its row, showing its field's label (the `label` of the field, a key such as `customerNameLabel`), so its inputs start at column 4. The label is a part named after its input, `nameBoxLabel` for `nameBox`, which code hides or disables as any part. A checkbox, a switch and a toggle label themselves, with their own caption or, without one, their field's label; a label written `for` an input the form labels is an error.

```clarity
form name customerForm at 1,1 span 12 rows 4 bind customer labels 3
    textbox name nameBox at 4,1 span 9 bind customer.name
    textbox name cityBox at 4,2 span 9 bind customer.city
```

A wizard is a `steps` part: its `step` blocks show one at a time, with the titles of all steps above and Clarity's own Back, Next and Finish below (the words `back`, `next` and `finish` in the language files). It needs at least two steps.

```xml
<!-- shop/design/NewOrder.cxaml -->
<Dialog class="shop.NewOrder" gives="Order">
  <Data name="order" type="Order" />
  <Steps name="wizard" at="1,1" span="12" rows="10" onnext="checkStep" onfinish="done">
    <Step title="@customerStep">
      <Dropdown at="1,1" span="6" bind="order.customer" />
    </Step>
    <Step title="@noteStep">
      <Textbox at="1,1" span="12" bind="order.note" />
    </Step>
  </Steps>
</Dialog>
```

A popover is a container outside the window's grid: it has no `at`, only a size, and `for` names the part it opens next to. It is closed until a click opens it (`on click -> filters`, from a button or a menu item) or code sets `filters.visible = true`; a click outside it or Escape closes it. It stands on the window, not inside another part.

```clarity
button @filterButton name filterButton at 1,7 span 2 on click -> filters
popover name filters for filterButton span 4 rows 2
    checkbox @onlyVinylCaption name vinylBox at 1,1 span 4 bind onlyVinyl on change: refilter
```

A slide-out is a container outside the window's grid too, as an IDE's side panels are: it has no `at` and no `for`. It slides in over the window from its right edge (`side left`: its left), as high as the window's content (below the menu bar, toolbar and ribbon, above the footer), with a shadow, and what is under it stays where it is. With `side top` or `side bottom` it slides down from the top of the content or up from its bottom, as wide as the content, as an IDE's bottom panel does. Its width is in pixels (420 without `width`, 100 to 1600); at the top or bottom it has a `height` instead (300 without it, 100 to 1600), and `min` and `max` limit that. `span` and `rows` give its own grid. Its caption shows in its header and on its handle: a tab on its edge of the window with the caption written along it (across at the top and bottom), always there, which slides it out with a click (or Enter) and back with another; the handles of one side stand one after the other in a strip the window's parts make room for; at the bottom that strip lies on top of the footer. A slide-out whose caption is written empty (`slideout ""`) has no handle. A click on another part can open it too (`on click -> details`, from a button or a menu item), and so can code (`details.visible = true`); Escape, a click beside it, the close button in its header or `details.visible = false` closes it. With `resizable` the person drags its inner edge between `min` and `max`, as a sidebar's (up and down at the top or bottom). With `pinnable` its header has a pin: pinned, it stays open beside the window's parts, which make room for it as for a sidebar, and a click beside it leaves it open (Escape closes it while the focus is in it). Code reads and sets `details.pinned`. The size the person dragged it to and whether it is pinned are kept per window for the next start. `on open` and `on close` run when it opens and closes, so a handler can load what it shows. The focus moves into it when it opens and back when it closes; a screen reader finds it as a dialog named by its caption. It stands on the window, unless it lies beside a part: `beside chapterTable` lays it against that part instead of the window's edge, at its `side` of it (`side right` at the part's right edge, sliding outward over what lies beside the part, up to the window's edge; with too little room there it lies over the part from that edge). Its handle then sits just outside that edge, and what lies beside the part (in its grid or the next pane) makes room for it, as the window's parts do for the handles at its edge; at the window's edge it sits just inside the part. The slide-out is as high as the container the part stands in (at the top or bottom, as wide), with a lighter shadow than one at the window's edge. It is written in the same container as that part, so it stands in a panel, a splitter pane, a tab, a section or a view: `slideout @chapterCaption name chapterProps side right beside chapterTable pinnable`. Pinned, it stays open over what lies beside the part; nothing makes room for it. A part that is not shown hides its handle.

```clarity
button @detailsButton name detailsButton at 1,1 span 2 on click -> details
slideout @orderDetails name details width 480 resizable min 300 max 900 pinnable span 6 rows 10 on open: loadDetails
    textbox @noteCaption name noteBox at 1,1 span 6 bind order.note
```

A slide-out is no taller than its window: its `rows` fit in the rows the window's height holds under its bars (see `width` and `height` of a window); an edit cuts it to fit, and the checker refuses one written taller by hand. Its rows take no room in the window's content, so it never makes the window scroll.

An accordion shows its sections' titles one under the other, with the open section's parts under its title; the first is open to start. A `horizontal` one stands its titles side by side as tall strips, read downwards, with the open section's parts beside its title, as a side panel of folding sections does. A press on a title opens that section, or folds it when it was the open one, and runs `on change`; code reads and sets the open section as `faq.open` (0 folds them all). A section's header can have a color of its own, so each stands out: `section @chaptersSection color "#3b6ea8"`, written out or a color of the theme by name (`color accent`); its text is black or white, whichever reads best, and it darkens a little under the pointer. A section without one takes the theme's look. A scroll box holds parts on its own grid like a panel, and parts placed below its rows scroll into view.

A button with `split` has an arrow beside it that opens its `menu` block, written as a context menu is; the button itself runs its `on click`. A toggle is a button that stays pressed: bound to a bool as a checkbox is, or holding its own, which code reads and sets as `part.checked`; its `on change` runs when the person presses it.

```clarity
toggle @boldCaption name boldToggle icon "bold" at 1,1 span 2 bind note.bold on change: restyle
buttons name alignButtons at 3,1 span 3
    toggle name leftToggle icon "text-align-start" at 1,1 span 1 on change: alignLeft
    toggle name rightToggle icon "text-align-end" at 2,1 span 1 on change: alignRight
button @saveCaption name saveButton split at 7,1 span 2 on click: save
    menu
        item @saveAsItem name saveAsItem on click: saveAs
```

Any part can have a context menu: a `menu` block indented under it, with items written as the frame's menu items are (`item`, `separator`, submenus, `on click: handler` or `on click -> Target`). A right click opens it at the pointer; on a table or list it chooses the row under the pointer first, so the handler finds it as `grid.selected`. On a canvas the handler finds where it was opened as `canvas.x` and `canvas.y`, in the canvas's units, to act on the box there, and the line it was opened on as `canvas.bent` (its name, empty on a box or the paper) with its points as `canvas.via`, so a **Remove line** item knows which; the line is taken in hand as a press on it takes it. Named items hide and disable as any part does.

```clarity
table name grid at 1,1 span 12 rows 6 bind albums columns title
    menu
        item @openItem name openItem on click: openClicked
        separator
        item @deleteItem name deleteItem on click: deleteClicked
```

Next runs the handler of `on next` on the step the person is on, then moves one step on. A `refuse` in it keeps the person on the step and says why; a handler that sets `wizard.step` itself decides where the wizard goes. Without `on next`, Next just moves on. Back goes one step back and runs nothing, so what was entered stays. Finish, on the last step, runs `on finish`, which saves, closes the dialog with its value (`close with order`), or does what the wizard is for.

### Sound

A sound part plays the project's sound files and makes tones and melodies (DESIGN 5.22). It is heard, not seen: it stands on the window without a place in its grid, and the designer keeps it in a tray under the window. Code tells it what to play, as it tells a custom part:

```xml
<!-- dice/design/Table.cxaml -->
<Window class="dice.Table">
  <Content name="main" at="1,1" span="12">
    <View name="TableView" />
  </Content>
</Window>
```

```xml
<!-- dice/design/TableView.cxaml -->
<View class="dice.TableView">
  <Sound name="rattle" at="1,1" span="1" source="roll.wav" volume="80" />
  <Sound name="fanfare" at="1,1" span="1" />
</View>
```

```clarity
code TableView
    handler rollClicked
        rattle.play()

    handler diceLanded(total: integer, faces: text)
        if total == count * 6
            fanfare.melody("C5/2 E5/2 G5/2 C6*2", 160)
```

| Code | What it does |
|---|---|
| `rattle.play()` | plays its `source` from the start; over and over with `repeat` |
| `rattle.stop()` | stops everything it plays |
| `fanfare.tone(440, 0.5)` | a tone of 440 hertz for half a second |
| `fanfare.melody("C4 E4 G4", 120)` | the notes, at 120 beats a minute |
| `rattle.volume(50)` | how loud it plays from now on, 0 to 100 |

- A note is a letter `A` to `G`, then `#` or `b`, then an octave `0` to `8`, 4 unless given: `C4`, `F#3`, `Bb`. `-` is a rest. Each lasts a beat; `*2` makes it two beats, `/2` half a beat. A melody written in the code is checked; one made while the program runs skips the words that are not notes.
- A sound file is a `.wav`, `.mp3`, `.flac` or `.ogg` file of the resources folder: those every stack plays. Its resource view in Clarity plays it.

**Songs** (docs/AUDIO-PLAN.md). A sound plays a song from disk too, and code follows it as it plays:

```xml
<!-- player/design/Player.cxaml -->
<Window class="player.Player">
  <Content name="main" at="1,1" span="12" rows="2">
    <View name="PlayerView" />
  </Content>
</Window>
```

```xml
<!-- player/design/PlayerView.cxaml -->
<View class="player.PlayerView">
  <Data name="played" type="integer" />
  <Sound name="music" at="1,1" span="1" onend="songEnded" onfail="songFailed" />
  <Timer name="clock" at="1,1" span="1" every="0.5" ontick="clockTicked" />
  <Slider name="progress" at="1,2" span="8" bind="played" max="100" min="0" onchange="progressChanged" />
</View>
```

```clarity
code PlayerView
    handler playClicked
        music.play(chosen.path)

    handler progressChanged
        music.seek(played)

    handler clockTicked
        played = music.position
```

| Code | What it does |
|---|---|
| `music.play(path)` | plays the song at `path` (mp3, flac, ogg, wav), read as it plays |
| `music.pause()`, `music.resume()` | holds the song where it is, and plays on |
| `music.seek(90)` | moves the song to 90 seconds from its start |
| `music.playing`, `music.paused` | whether a song plays now, or is held |
| `music.position`, `music.length` | whole seconds played, and the song's length in whole seconds (0 when its file does not say) |
| `music.file` | the path of the song that plays; empty for none or a resource |
| `music.problem` | why the last song did not play; empty when it did |
| `music.title`, `music.artist`, `music.album`, `music.track` | what the playing song's file says of it (ID3 in mp3, Vorbis comments in flac and ogg); empty, and 0 for the track, when it does not say |
| `on end: handler` | runs when a song played to its end; not when stopped |
| `on fail: handler` | runs when a song cannot play (missing, not a sound); `problem` says why |

What a sound says is read, not set. On the desktop songs play through Clarity.Audio, which Clarity adds to a program that plays them; in a browser the page plays a song of the resources, and a file of the computer's disk cannot play there (`on fail`).

### Video

A video part plays a video in its place in the grid (docs/VIDEO-PLAN.md): an `.mp4` of the resources (`source`) or a file from disk, told by code as a sound plays songs, and shown as large as fits, centred, on black.

```xml
<!-- cinema/design/Screen.cxaml -->
<Window class="cinema.Screen">
  <Content name="main" at="1,1" span="12" rows="6">
    <View name="ScreenView" />
  </Content>
</Window>
```

```xml
<!-- cinema/design/ScreenView.cxaml -->
<View class="cinema.ScreenView">
  <Video name="film" at="1,1" span="8" rows="6" source="intro.mp4" volume="80" onend="filmEnded" onfail="filmFailed" />
</View>
```

- Code tells it `play()` (its source), `play(path)`, `pause()`, `resume()`, `seek(seconds)`, `stop()` and `volume(level)`, and reads `playing`, `paused`, `position` and `length` (whole seconds), `file` and `problem`; what it says is read, not set.
- `on end` runs when it played to its end, `on fail` when it could not play.
- Its pictures come from a codec the project added, the system's own decoder, or Cisco's openh264 carried beside the program; its sound plays through Clarity.Audio, and the pictures follow it.

### Custom parts

A custom part is a part an outside library draws: a game scene drawn with SFML, a map, a code editor. It is declared in a module, so the checker, code and the designer treat it as a built-in part, and placed in a window by its name. DESIGN 5.19 and 6.5 have the whole design; the `orbs` example has one in C++, drawn on Clarity.UI's canvas.

```clarity
part Arena
    native "parts/Arena"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    tells scored(points: integer)
    hears restart(level: integer)
    size 8 x 6
    grows
    preview "arena.png"
```

| Line | What it says |
|---|---|
| `native "folder"` | C or C++ in that folder with its `CMakeLists.txt`, using SFML, SDL or anything CMake finds. The C and C++ stacks. |
| `crate "folder"` | A Rust crate in that folder, using any crate (Bevy, wgpu, softbuffer). The Rust stack. |
| `go "folder"` | A Go package in that folder with its `go.mod`, using any module. The Go stack. |
| `nuget "Package" version` | A control from a .NET library on NuGet. The C# stack; not generated yet, it comes with Clarity.UI's custom controls. |
| `dll "file"` | A control from a .NET assembly in the project. The C# stack; not generated yet, it comes with Clarity.UI's custom controls. |
| `binds type` | The type of value it shows and edits, which `bind` gives it. Without it the part binds nothing. |
| `name: type, default v, min a, max b` | A property, set in the window and in code. Text, a number, a date, a duration, an enum or bool. |
| `tells name(values)` | One of its events, with typed values. |
| `hears name(values)` | Something code tells it to do. |
| `size n x n` | The span and rows the Parts palette drops it with. |
| `grows` | Placed in a window, it takes the height the window has left, as a table or a canvas does (see [The grid](#the-grid)). Without it, it keeps the height of its rows. The designer's Grows with the window sets it. |
| `preview "file"` | Its picture on the design canvas, from the resources; without it a box with its name. |

A part's name starts with a capital, which no built-in part does. Its properties cannot be named after a word every part has: `at`, `span`, `rows`, `bottom`, `bind`, `name`, `on`, `tooltip`, `taborder`, `visible`, `enabled` or `value`.

```xml
<!-- game/design/Game.cxaml -->
<Window class="game.Game">
  <Content name="main" at="1,1" span="12" rows="6">
    <View name="GameView" />
  </Content>
</Window>
```

```xml
<!-- game/design/GameView.cxaml -->
<View class="game.GameView">
  <Data name="player" type="Player" />
  <Custom name="arena" at="1,1" span="8" rows="6" bind="player.best" part="Arena" values="{&quot;speed&quot;:{&quot;kind&quot;:&quot;number&quot;,&quot;value&quot;:&quot;5&quot;},&quot;title&quot;:{&quot;kind&quot;:&quot;str&quot;,&quot;value&quot;:&quot;@levelOne&quot;}}" tells="[{&quot;handler&quot;:&quot;arenaScored&quot;,&quot;tell&quot;:&quot;scored&quot;}]" onchange="bestChanged" />
</View>
```

```clarity
code GameView
    handler arenaScored(points: integer)
        message @scored(points = points)
        arena.speed = arena.speed + 1
        arena.restart(2)
```

- A property is set on the part's line (`speed 5`) and read and set in code (`arena.speed`). A text property's text is shown, so it is a key like a caption.
- `on change` runs when the person changed the value it binds; `on scored` runs when the part tells it, and the handler takes the tell's values.
- `arena.restart(2)` gives the part a hear with its values.
- `visible` and `enabled` are every part's: code hides and disables a custom part as any other.

A part is written in the project's own language: a library line of another language is an error at the part, so a project that changes stack sees each part to write again.

**C, C++, Rust or Go.** The part's folder holds its sources: a `CMakeLists.txt`, a crate's `Cargo.toml` or a module's `go.mod`. Clarity writes one file of it from the declaration, and writes it again when the declaration changes: `clarity/Arena.h` (C functions), `src/clarity.rs` (the trait `ArenaPart` and `ArenaHost`, whose methods are the tells) or `clarity.go` (the interface `Part`, which the package's `New` makes, and `Host`). With Bevy, run its App without its own window loop and call `app.update()` in `frame`.

Build and Run write the interface file and, for a new part, a starter (`clarity parts shop.clarity` does it from the command line), and build the part with the program: a C or C++ part with CMake into `libclarity_parts` beside the program, which it loads; a crate as a dependency of the program's; a Go module required by the program's. The part draws in a native child window at the part's place; it runs on Linux with X11 (a program with such a part starts under XWayland on a Wayland desktop), and it is called on a thread of its own, its frame about 60 times a second. While a Clarity dialog, popover, menu or open list lies over the part, the part steps aside. `CLARITY_PARTS_TRACE=1` prints where each part is placed.

**A .NET library** (`nuget` or `dll`, the C# stack only) is shown through an adapter in `parts/Notes/Notes.cs`, half of a partial class: it holds the library's control and writes `ShowValue`, a `Show<Property>` per property and an `On<Hear>` per hear, which the half Clarity generates calls; it calls a tell by its name (`CaretMoved(3)`) and `Changed(value)` when the person changed the value. Declaring the part writes a starter adapter; one that no longer fits the declaration does not compile.

### External libraries

A library of the project's own language, called from code (DESIGN 5.21). It is declared in a module with its package and the functions code calls; an adapter in `libraries/Words/`, in the project's language, writes them with the package.

```clarity
library Words
    nuget "Humanizer.Core" 2.14.1
    function spell(n: integer) -> text

action describe(order: Order)
    effects db.write
    order.words = Words.spell(42)
```

| Line | Stack |
|---|---|
| `nuget "Package" version`, `dll "file"` | C#: the adapter `Words.cs` is the other half of the static partial class `Words.g.cs` declares |
| `crate "name" "version"` | Rust: a crate whose `Words` implements the trait `WordsLibrary` of `src/clarity.rs` |
| `go "module" version` | Go: a module whose package has the functions `clarity.go` types |
| `cmake "Package"` | C, C++: a CMake target `Words` with the functions of `clarity/Words.h` or `.hpp` |

Code calls a function as `Words.spell(42)`, checked like an engine's. Values in and out are text, a number, a date, a duration or bool. A library of another language than the project's is an error, and declaring one writes a starter adapter (Build, Run, or `clarity parts shop.clarity`).

## 7. The app and its frame

A project has one app, in a file of its own beside the windows: `design/<App>.cxaml`, whose root `<App class="shop.Shop">` names its module and its name. It names the main window and declares the frame every window shares: a menu, a toolbar, a ribbon, sidebars and a footer. Its code is the block `code Shop` of its module.

```xml
<!-- shop/design/Shop.cxaml -->
<App class="shop.Shop" title="@shopTitle" main="CustomerCard" onstart="appStarted">
  <Data name="session" type="Session" />
  <Menu name="mainMenu">
    <Item caption="@fileItem" name="fileItem">
      <Item caption="@newCustomerItem" name="newCustomerItem" key="Ctrl+N" opens="CustomerForm" />
      <Item kind="separator" />
      <Item caption="@quitItem" name="quitItem" key="Ctrl+Q" opens="quit" />
    </Item>
  </Menu>
  <Toolbar name="mainToolbar">
    <Item kind="button" caption="@customersButtonCaption" name="customersButton" icon="list" opens="CustomerBrowse" />
    <Item kind="separator" />
    <Item kind="button" caption="@ordersButtonCaption" name="ordersButton" onclick="ordersClicked" />
  </Toolbar>
  <Ribbon name="mainRibbon" collapsible="true">
    <Tab title="@homeTab" name="homeTab">
      <Group title="@customersGroup" name="customersGroup">
        <Item kind="button" caption="@newButtonCaption" name="newButton" icon="plus" large="true" opens="CustomerForm" />
        <Item kind="button" caption="@browseButtonCaption" name="browseButton" icon="list" opens="CustomerBrowse" />
      </Group>
    </Tab>
  </Ribbon>
  <Sidebar side="left" name="leftSidebar" width="192">
    <List name="recentList" at="1,1" span="3" rows="10" bind="recent" show="name" />
  </Sidebar>
  <Footer status="@shopStatus" columns="12">
    <Text name="userText" at="10,1" span="3" bind="session.user" />
  </Footer>
</App>
```

### App settings

The app's settings are attributes of its root; its data are `Data` elements, the first things inside it.

| Setting | Meaning |
|---|---|
| `title="@key"` | The application's title: what a window without a title of its own shows in its title bar. A key may hold `{app.version}`. |
| `main="Window"` | The window shown at start; it must be a window. |
| `splash="Page" splashFor="2"` | Optional: the [splash screen](#splash-screen) shown while the application starts, at least the seconds `splashFor` gives (0.1 to 30). |
| `<Data name="session" type="Session" />` | Data for the parts in the sidebars and the footer. |
| `onstart="handler"` | Runs when the application starts, before its main window opens. Its code is in `code AppName`. |
| `onunconnected="handler"` | Runs when the databases did not open, such as a server that is down or a [secret setting](#secret-settings) not kept yet, instead of the main window; `handler notConnected(reason: text)` gets why, in the person's language (Clarity's texts `unconnectedNotKept` and `unconnectedFailed`). What it opens shows first. Closing the last window, or the desktop window from its title bar, then connects again if a setting changed, and the main window opens once it does; closing it unchanged ends the application. |

An app written as an `app` block in a module's `.clarity` file is an error; **Move pages into design** in Problems, or `clarity convert-pages`, moves it into its own file, as it does a window.

### Regions

| Region | Docks | Holds |
|---|---|---|
| `<Menu>` | at the top | `<Item>` elements, which can hold more items inside them (a submenu), and `<Item kind="separator" />` |
| `<Toolbar>` | at the top, below the menu | `<Item kind="button">` and separators |
| `<Ribbon>` | at the top | `<Tab title="@key">` elements with `<Group title="@key">` elements of buttons and separators |
| `<Sidebar side="left">` or `side="right"` | at that side, `width` pixels wide (240 without it) | parts, on its own grid of `columns` (3 without it) |
| `<Footer>` | at the bottom | an optional `status="@key"` and parts on a one-row grid of 12 columns |

Every menu item and button has a name, in camelCase like a part's (`<Item caption="@newCustomerItem" name="newCustomerItem">`); a separator has none. Its name is unique among the window's parts, including the menu and the toolbar themselves. The designer names a new one after its caption and kind, as `newCustomerItem`, which keeps it apart from data and parts of the same word.

The menu and the toolbar are named parts: `<Menu name="mainMenu">`, `<Toolbar name="mainToolbar">`. Code hides one with its `visible` and turns it off with its `enabled`.

A ribbon is a named part too, and so are its tabs and groups: `<Ribbon name="mainRibbon" collapsible="true" collapsed="true">`, `<Tab title="@homeTab" name="homeTab">`, `<Group title="@clipboardGroup" name="clipboardGroup">`. With `collapsible` the person double-clicks a tab to fold the ribbon to its tabs, and opening a tab unfolds it; `collapsed` starts it folded. Code reads and sets the open tab as `part.tab` (from 1; a hidden tab gives way to the first visible one) and the folding as `part.collapsed`, and hides a tab or group with its `visible`.

A sidebar is a named part: `<Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" collapsed="true" maximizable="true" columns="4">`. Its width is in pixels (240 without `width`, 100 to 1600). With `resizable` the person drags its inner edge between `min` and `max`; with `collapsible` a button folds it to a narrow strip and back, and `collapsed` starts it folded; with `maximizable` a button lets it fill the window. `columns` gives its own grid (3 without it). Code reads and sets `part.collapsed` and `part.maximized`.

A window can declare its own regions, with the same elements in its own `.cxaml` file:

```xml
<Window class="todo.Users" title="@usersTitle" onopen="usersOpened">
  <Menu name="usersMenu">
    <Item caption="@fileItem" name="usersFileItem">
      <Item caption="@mainItem" name="mainItem" opens="close" />
      <Item kind="separator" />
      <Item caption="@languageItem" name="usersLanguageItem" opens="language" />
    </Item>
  </Menu>
  <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" columns="4">
    <List name="userList" at="1,1" span="4" rows="10" bind="users" show="name" />
  </Sidebar>
  <Content name="main" at="1,1" span="12" rows="11">
    <View name="UsersView" />
  </Content>
</Window>
```

A window's region replaces the app's region of the same kind, and a window's sidebar replaces the app's sidebar on the same side. The other regions still come from the app.

### Commands

A link does one of three things: opens the address in `to`, opens the address it binds (an email address to write to), or runs its `on click`. The checker wants exactly one, and a whole address in `to` (`"https://lucide.dev"`, not `"lucide.dev"`).

Icons are [Lucide](https://lucide.dev)'s, by name: `icon "save"`, `icon "trash"`, `icon "triangle-alert"`. A button, a menu item and a toolbar or ribbon button show one beside their caption, and the `icon` part shows one on its own. The checker knows every name and suggests the nearest for a misspelt one; the designer offers them all. An application carries only the icons it uses, with Lucide's license in its `THIRD_PARTY_NOTICES.md`.

Menu items and toolbar and ribbon buttons are commands, `<Item>` elements. Their attributes:

| Attribute | On | Meaning |
|---|---|---|
| `kind` | toolbar and ribbon | `button`, or `separator` (also in a menu). A menu's items have no kind. |
| `caption="@key"` | all | The text. A menu item needs one; a button needs a caption or an icon. |
| `name` | all | The command's name, for handlers. |
| `icon="name"` | all | An icon, by its name among Clarity's icons. |
| `key="Ctrl+N"` | all | The key that runs it: shown in the menu, or in the button's tooltip. A ribbon button's works whichever tab is open. One key runs one command in a frame; the window's own buttons go first. |
| `large="true"` | ribbon buttons | A large button. |
| `opens="Target"` | all | Opens a window (with `newWindow="true"`, in a desktop window of its own), or `close`, `quit`, `undo`, `redo`, `language`, `theme` or `keys` (the keys window, where the project file says `keys editable`). |
| `onclick="handler"` | all | Runs a handler. |

An item with a submenu opens the submenu, so it has no `onclick` or `opens` of its own.

### Themes

A theme says how the application looks, as named values rather than a style sheet, so every stack can carry it: the web renderer as a CSS file of variables, C# as the resolved theme in JSON (light and dark) that Clarity.UI lays over Clarity's own theme. It is declared in a module, and the project file names the themes the application offers: `theme Shop Night`, the first one the one it starts with.

Every theme starts from Clarity's own, the built-in `theme Clarity`, which sets every value below; a theme sets only what it changes, and the rest stays Clarity's. The Module view shows Clarity's own theme, and "New theme" starts from a full copy of it.

```clarity
theme Shop
    mode system
    color accent "#1f6f5c" dark "#4fb393"
    color brand "#f5c400"
    font "Inter"
    font heading "Georgia, serif"
    size 14
    size heading 20
    radius 4
    radius button 12
    padding 5
    shadow flat
    part button background panel, border accent, text accent
    part button hover background accent_soft
    part header background accent_soft, text accent
    style primary background accent, text on_accent, weight bold
    style quiet border none
```

| Line | Meaning |
|---|---|
| `title @nightOwlTheme` | The name people see when they choose the theme, by its key. A theme the application offers needs one. |
| `mode light`, `mode dark`, `mode system` | The theme is always light, always dark, or follows the system. |
| `color name "#rrggbb"` | A color, written `#rrggbb` or `#rgb`; `dark "#rrggbb"` after it gives its value in dark mode. Code in a memo takes `code_background` (read only) and the colors of its words, `code_keyword`, `code_type`, `code_text`, `code_number` and `code_comment`, and a diff's `code_added` and `code_removed`. |
| `font "Families"`, `font heading ...`, `font mono ...` | The fonts of text, headings and code, as a list of families: `"Inter, sans-serif"`. |
| `size n`, `size small n`, `size heading n` | Text sizes in pixels: the text, small text (help, captions) and headings. |
| `radius n`, `radius input n`, `radius button n`, `radius panel n` | Corner radii in pixels: of everything, or of one kind. |
| `padding n`, `border n` | The space inside inputs and buttons, and the width of borders, in pixels. |
| `shadow none`, `shadow flat`, `shadow raised` | The shadow under menus, popups and messages. |
| `part kind [state] look` | How a kind of part looks: `button`, `input`, `header` (of a table), `tab`, `menu`, `list` (its frame and rows), `card` (a row of a list with `cards`), `card_line` (the lines under a card's title) or `code` (a read-only memo of code, so people see it is code to read, not a box to type in); with `hover`, `pressed` or `focus`, in that state, and a list, card or card line with `selected`, as the row chosen, a tab with `selected` as the one open. A card's padding and the gap between cards follow the theme's `padding`. |
| `style name look` | A named style that parts pick with `style name`. |

A look (of a part or a style) sets `background`, `text` and `border` to a color of the theme by name or written out (a background or border can be `none`), `radius n`, `weight bold` or `weight normal`, and `size n`, the text size in pixels (6 to 200), separated by commas. A style also sets how its text shows: `font` with a font of the theme (`text`, `heading` or `mono`), and `align left`, `align center` or `align right`; aligned text keeps an input's padding from the edge. A calculator's display:

```clarity
theme Calculator
    color lcd "#c7d3a8" dark "#1d2618"
    style display background lcd, size 40, font mono, align right
```

The colors a theme can set by role, which the application itself uses:

| Role | Used for |
|---|---|
| `background`, `panel`, `raised` | the window, panels and inputs, raised surfaces such as buttons and table headers |
| `frame`, `frame_text` | the menu bar, toolbar, sidebars and footer |
| `text`, `muted` | text, and quieter text such as help and labels |
| `border`, `strong_border` | lines between things, and the borders of inputs and buttons |
| `accent`, `accent_soft`, `on_accent` | what is chosen, active or important, a soft tint of it, and text on it |
| `selection`, `focus`, `input` | a chosen row, the focus ring, the inside of inputs |
| `error`, `warning`, `success` and their `_soft` tints | what went wrong, what needs care, what went well |

Other names are colors of the theme's own, for its parts and styles.

- A part picks a style with `style primary`. Every theme the application offers must have that style, so switching themes never drops one; a theme that misses it is an error.
- The generator writes Clarity's own theme as `ui/themes/clarity.css` and each offered theme as a CSS file of its own, `ui/themes/shop.css`, linked from the application's page after it. The application holds the theme's class (`theme-shop`), and a part with a style holds its class (`style-primary`), as a web developer would write it. The application's own CSS reads only these values. The designer uses the same CSS for its canvas, and the Module view shows a sample window for the theme being written.
- An application that offers several themes lets people switch with `on click -> theme`: a menu item or button that lists the themes by their titles, switches at once and remembers the choice for the next start. The Project tab chooses the themes the application offers and the one it starts with, and can show the designer's canvas in any of them.

## 8. Code behind

A window's handlers are in a `code` block named after the window, in its module's `.clarity` file (the window's own `.cxaml` names them). The app's handlers are in a `code` block named after the app, and an entity's data events in one named after the entity (see [Data events](#data-events)).

```clarity
code CustomerForm
    handler kindChanged
        // Only business customers have a VAT number.
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        save customer
        message @savedName(name = customer.name)
        close
```

- `handler name` declares a handler. It runs when an event wired to it happens.
- `handler name(param: type, ...)` declares a handler that takes values, which a part gives it: `button "7" on click: digit(7)` runs `handler digit(d: integer)` with `d` 7. Each parameter a part gives is `text`, `integer`, `decimal`, `money`, `bool` or an enum; a handler that the answer to a question runs may also take dates, durations and records. The values must fit: as many as it has parameters, each of its type. Events of the window itself (`on open`) give no values, so their handlers take none.
- A handler that is wired but not written is an error. A handler that is written but not wired anywhere is a warning.
- Handlers run in the backend. The window sends its data with the event, the handler runs, and the window shows what changed: data, part properties, messages, and windows to open or close.
- **Fail safe.** A handler that stops on an error (a refusal, a record that is gone, a list position out of range, anything the runtime did not foresee) shows the error as an error message, and the application keeps working: the next handler runs as usual, on every stack. A part that cannot be drawn shows its name and the reason in its own place, and the rest of the window works.

### Undo

Every part the person edits takes back its own edits with Ctrl+Z, and does them again with Ctrl+Y (or Ctrl+Shift+Z), on every stack, with nothing to write:

- **What one edit is.** A click on a canvas, its drag and drop (from the first drag to the drop), a bent line, a new line drawn between boxes, a box resized by its grip, a table cell, a change of an input (bound or not), a picture loaded into an image or removed, a row moved in a part that reorders. A list or tree that does not reorder only selects, so it has no undo, nor a custom part, which brings its own, nor a canvas's `on pointer` and `on key`. Its undo puts back the window's data as it was before the edit, and what the handler of that edit (`on drop`, `on bend`, `on connect`, `on resize`, `on change`, `on reorder`) saved or deleted, cascades included.
- **Where.** Ctrl+Z acts on the part with the focus. In a `form` or a `panel`, it walks back the fields one by one, the last first. In a text input, its own text undo comes first while it has something to take back.
- **A menu item or a button** reaches it with the built-in targets `undo` and `redo`: `item @undoItem name undoItem key "Ctrl+Z" on click -> undo`. They act on the part that had the focus last.
- **Not taken back:** moving around (a tab, folding, zoom and pan, the selected row), what a click or a timer did, and effects outside the databases and the window (files written, messages, the clipboard, calls to other services).
- **`no undo`** on a part's line switches it off there.
- **The whole window.** An undo also puts back what the edit's handler set on parts: a caption, visible, enabled, readonly, an error, and each value of a custom part (`board.count = count` in a slider's `on change`), value by value.
- **Safe.** When what an undo would take back was changed since (by another edit, window or person), it is refused with a message (Clarity's word `undoChangedSince`) and changes nothing. It compares the record's version where its entity has one (see [Entities](#entities)), the whole row where it has none.

### What a handler sees

- **The window's data**, by its `data` names: `customer`, `orders`.
- **The window's parts**, by their names: `save`, `vatNumber`.
- **Its own locals**, made with `let`.
- **Enum values**, written bare (`Business`) where the expected type says which enum is meant, and qualified (`CustomerKind.Business`) where it does not.
- **Functions and actions** of its own module and of the modules it uses.

A name that is both data and a part in the same window is an error.

### Part properties

| Property | On | Type | Can be set |
|---|---|---|---|
| `visible` | every part | bool | yes |
| `enabled` | every part | bool | yes |
| `caption` | labels, help texts, headings, buttons, commands | text | yes |
| `error` | inputs | text | yes; `""` clears it |
| `readonly` | inputs | bool | yes |
| `text` | text boxes, memos and password inputs that are not bound | text: what the user typed | yes |
| `checked` | checkboxes and switches that are not bound | bool: whether it is on | yes |
| `selected` | tables, lists, trees | the record of the selected row; with none chosen an empty record, as `new` makes (its `id` is 0), so `if grid.selected.id == 0` asks first | yes: the first row that is that record (a stored record by its id, a record by its values), or `none` for no row |
| `folded` | foldable panels and forms | bool: whether it is folded, by the person or by code | yes |
| `collapsed` | collapsible sidebars | bool: whether it is folded to its button | yes |
| `maximized` | maximizable sidebars | bool: whether it fills the window | yes |
| `tab` | ribbons | integer: the open tab, counted from 1 | yes |
| `step` | wizards (`steps`) | integer: the step shown, counted from 1 | yes |
| `open` | accordions | integer: the open section, counted from 1; 0 when none is open | yes |
| `collapsed` | collapsible ribbons | bool: whether it is folded to its tabs | yes |
| `caret` | text boxes and memos | integer: where the caret is, as how many characters stand before it (0 at the start) | yes: the caret moves there, selecting nothing |
| `selectedText` | text boxes and memos | text: what the person selected; empty when nothing is | no |

A text box or memo takes text from code where the person is: `manuscriptBox.insert(text)` puts it where the selection is (at the caret when nothing is selected), replacing the selection, as one step Ctrl+Z takes back; it goes to the field the memo binds, without its `on change`. So a tool inserts at the caret: `manuscriptBox.insert("> " + manuscriptBox.selectedText)` turns the selection into a quote. A memo can have a toolbox of such tools, a page of its own as a canvas's is: `<Memo name="manuscriptBox" ... markdown="true" toolbox="caret" toolboxPage="WritingTools" />` with a `<Toolbox class="novel.WritingTools">` file holding tools with `onclick` (`<Tool name="quoteTool" at="1,1" span="1" icon="quote" onclick="quoteClicked" />`), buttons and labels; it comes at the caret: Ctrl+Space calls it up just under the caret, and a right-click moves the caret where it was pressed and calls it up there, with the memo's Cut, Copy and Paste as its last row. A tool's handler runs on the text where the person is, then the toolbox goes; a press beside it or Escape closes it too. Written `toolbox="popup"` it floats over the window instead, or `toolbox="slideout"` against the memo's edge, while the memo may be written in (in a markdown memo while it shows Write). Painting tools (`does`, size, colors, layers) are a canvas's.

A passage put in from elsewhere can keep where it came from as a link into the application: `manuscriptBox.insert("[" + f.text + "](clarity:fragment/" + f.id + ")")`. The preview, or a markdown part, shows it as a link; a click on it opens no browser but runs the part's `on link` handler, which reads the address as `manuscriptBox.link` (`clarity:fragment/412`) and goes there.

A bound input is changed through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`.

### Asking the person

A handler can ask the person something: a yes or no, a text, a number, a day, a value of an enum or a record. The question shows in Clarity's own dialog when the handler has finished, never one of the system or the browser, and the answer runs another handler:

    handler deleteClicked
        ask @deleteAlbum(title = album.title) yes @delete no @keep then deleteConfirmed

    handler deleteConfirmed
        delete album

    handler lendClicked
        ask @lendTo for Friend from friends then lend(album)

    handler lend(what: Album, friend: Friend)
        what.copies = what.copies - 1
        save what

- **What is asked decides the dialog.** `yes @word no @word` asks yes or no; the handler runs on yes and takes no answer (without `yes` and `no` the buttons say OK and Cancel). `for text`, `for integer`, `decimal`, `money`, `date`, `datetime` or `duration` asks for a typed value; `default value` fills it first. `for` an enum offers its values with their labels. `for` an entity offers its records to choose from; `from list` offers the records of a list instead.
- **The answer** is the last value of the handler after `then`. Values written in the call come first (`then lend(album)`), which is how what the asking handler knew reaches the answering one, as it was when it asked. The answer's type must be the type of the handler's last parameter.
- **A dialog window of your own** answers too. A dialog that gives a value says its type, and its handlers close it with one:

      <!-- design/PickFriend.cxaml -->
      <Dialog class="loans.PickFriend" gives="Friend">
        <Data name="friends" type="list&lt;Friend&gt;" />
        <Table name="grid" at="1,1" span="12" rows="6" bind="friends" columns="name" />
        <Button name="okButton" at="1,7" span="2" caption="@ok" onclick="picked" />
      </Dialog>

      // in the module
      code PickFriend
          handler picked
              close with grid.selected

  `ask PickFriend then lend(album)` opens it, as `open` does, and what it closes with is the answer; `ask PickFriend with friends = mine then lend(album)` fills its data first. Only a dialog gives a value, every `close with` in it gives the type it says, and the answer's type is that type. A dialog closed without a value runs nothing.
- **Cancelling** (no, Escape, a click beside the dialog, or a dialog closed without a value) runs nothing.
- **When.** The statements after `ask` still run; the dialog shows when the handler has finished, with whatever else it changed. A handler that asks twice asks one question after the other. A handler that asks and closes its window is an error: the answer would have nowhere to go.
- **Where.** Only handlers ask. A function has no effects, and an action may run where nobody is at the screen.
- The question and the words of its buttons are keys, as every text people read.

### Drawing on a canvas

A canvas is drawn by its draw handler, wired with `on draw`. The handler draws shapes in the canvas's own units (`units 200 x 100`, 100 x 100 without it), x to the right and y down; the canvas scales them to fit the part, in proportion and centered.

```xml
<!-- dial/design/Main.cxaml -->
<Window class="dial.Main">
  <Content name="main" at="1,1" span="12" rows="8">
    <View name="MainView" />
  </Content>
</Window>
```

```xml
<!-- dial/design/MainView.cxaml -->
<View class="dial.MainView">
  <Data name="speed" type="integer" default="80" />
  <Canvas name="dial" at="1,1" span="12" rows="7" none="surface" ondraw="drawDial" />
  <Button name="faster" at="7,8" span="6" caption="@fasterCaption" key="+" onclick="change(10)" />
</View>
```

```clarity
code MainView
    handler drawDial
        let angle = 210 - speed * 240 / 200
        dial.clear("panel")
        dial.ring(50, 50, 42, "border", 3)
        dial.line(50, 50, 50 + 34 * cos(angle), 50 - 34 * sin(angle), "accent", 3)
        dial.text(50, 72, "{speed} km/h")

    handler change(by: integer)
        speed = speed + by
```

| Shape | Values | What it draws |
|---|---|---|
| `canvas.clear(color)` | the canvas's `background` when left out | Fills the whole canvas; on a canvas with `zoom`, the whole part, which has no edges. |
| `canvas.line(x1, y1, x2, y2, color, width)` | width 1 when left out | A line with round ends. |
| `canvas.rect(x, y, width, height, color)` | | A filled rectangle. |
| `canvas.circle(x, y, r, color)` | | A filled circle around x, y. |
| `canvas.ring(x, y, r, color, width)` | width 1 when left out | A circle's outline. |
| `canvas.text(x, y, text, color, size, align, style)` | color `"text"`, size 10, align `"center"` and no style when left out | Text centered on x, y, or with `"left"` starting at x and with `"right"` ending there; slots are filled in the person's format, like a message. `style` is any of `"bold"`, `"italic"` and `"mono"` (the theme's code font), together as `"bold mono"`. |
| `canvas.link(name, x1, y1, x2, y2, via, color, width, start, end, label, style)` | width 1, plain ends, no label and solid when left out | A line people bend, from x1, y1 through its bend points to x2, y2. `via` is a `list<integer>` of the points, x, y, x, y, ... (`[]` for straight); `name` is plain text the code knows it by, as `"a{association.id}"`. See `on bend` below. An end that lies in a box (as `on connect` finds boxes) is drawn at the edge of that box, so a line from the middle of one box to the middle of another runs between their edges; while an object is dragged (`mode objects`), an end that lies in its box moves with it, and its bend points stay. `start` and `end` say what its ends show: `"arrow"`, `"triangle"` (hollow, a base type), `"diamond"` and `"filledDiamond"` (a whole), or a data model's `"one"`, `"many"` (a crow's foot), `"zeroOrOne"`, `"zeroOrMany"` and `"oneOrMany"`. `label` is a text halfway along it, in quiet italics, which moves with it. `style` is `"dashed"` or `"dotted"`. The checker knows the words. |
| `canvas.image(x, y, width, height, picture)` | | A picture of the resources folder by its name (`"pill.svg"`, or a picture constant), stretched to fill the box whose top left corner is x, y. |
| `canvas.ellipse(x, y, rx, ry, color)` | | A filled ellipse around x, y, rx across and ry high from its center. |
| `canvas.path(points, color, width)` | width 1 when left out | An open line through its points, a `list<integer>` x, y, x, y, .... |
| `canvas.object(record, x, y, width, height)` | | On a canvas in `mode objects`: the box of an object, the record of its list. It draws nothing itself; the shapes drawn after it, until the next `object`, are the object and move with it while it is dragged. |

- A color is a color of the theme by name (`"accent"`, `"text"`, a color the theme declares) or written out (`"#101810"`); a name the theme does not have is its text color.
- The draw handler runs when the window opens and after every other handler of the window, each time on an empty drawing, so the canvas always shows the data as it is. Only a canvas's own draw handler draws on it, and the handlers it calls that nothing else runs: `draw_mark(5, 100, 100)` can draw one square of a board. A handler that an event or another handler also runs cannot draw, since its shapes would land outside a drawing.
- `on click: handler` on a canvas runs the handler when the canvas is clicked, with `canvas.x` and `canvas.y` saying where, in the canvas's units, rounded down. They are read, not set, and 0 before the first click. A click beside the drawing, where the part shows none of it, runs nothing. The window draws the canvas anew after the handler, as after any other.

```xml
<!-- noughts/design/Game.cxaml -->
<Window class="noughts.Game">
  <Content name="main" at="1,1" span="12" rows="8">
    <View name="GameView" />
  </Content>
</Window>
```

```xml
<!-- noughts/design/GameView.cxaml -->
<View class="noughts.GameView">
  <Data name="status" type="text" />
  <Canvas name="board" at="1,1" span="12" rows="8" units="{&quot;height&quot;:300,&quot;width&quot;:300}" none="surface" ondraw="drawBoard" onclick="boardClicked" />
</View>
```

```clarity
code GameView
    handler drawBoard
        board.clear("panel")

    handler boardClicked
        let square = 1
        if board.x >= 200
            square = 3
        else if board.x >= 100
            square = 2
        if board.y >= 200
            square = square + 6
        else if board.y >= 100
            square = square + 3
        status = "Square {square}"
```

- `zoom` on a canvas lets the person zoom and pan its drawing, as Clarity's own diagram designer does, for a drawing larger than the part shows well, such as a diagram. It starts whole, fitted in proportion. The wheel pans (with Shift sideways), Ctrl (Cmd on macOS) with the wheel or a touchpad's pinch zooms around the pointer, and three buttons in its top right corner zoom out, show the zoom it is at (a click shows it whole again) and zoom in, named in Clarity's words `zoomOut`, `zoomWhole` and `zoomIn`. It zooms from a quarter of its size up to three times (further when the whole drawing is smaller or larger than that), and pans past its edges. The view stays while the canvas is drawn anew. `canvas.x` and `canvas.y` stay in the canvas's units, whatever is in view.
- `on drag: handler` runs while the pointer moves on the canvas with its button down, and `on drop: handler` once when it is let go. Both read where the pointer is (`canvas.x`, `canvas.y`, at the canvas's nearest edge when it is beside it) and where it was pressed (`canvas.fromX`, `canvas.fromY`), in the canvas's units. A press that does not move is a click. The handler runs with the latest place: moves that come while it runs are not queued one by one. The canvas is drawn anew after each, so code that moves what it draws by `canvas.x - canvas.fromX` shows it moving; the drop is the place to save. On a canvas with `zoom`, a drag pans instead when there is no drag handler, or when it starts on empty paper; a press on something drawn (a box, a circle, an ellipse, a ring, a picture or an object; not lines or text) is the handler's drag, whatever the handler does with it, as in Clarity's diagram designer. A panning drag has no drop.
- `on drop` without `on drag`: the canvas carries a box itself, as Clarity's diagram designer does. A press on a box (as `on connect` finds boxes, below) and a drag move the box and everything drawn inside it with the pointer, by whole grid steps when the canvas has `snap`, and the end of a link that lies in the box comes along; no handler runs while it moves, so a large drawing moves as smoothly as a small one. When the person lets go, `on drop` runs once with where it was pressed (`canvas.fromX`, `canvas.fromY`) and where that point was carried to (`canvas.x`, `canvas.y`), so the code moves the box it was pressed on by `canvas.x - canvas.fromX` and saves it. The canvas shows the box where it was put until the drawing comes back. Let go where it was, nothing runs. A drag on empty paper pans a canvas with `zoom`.
- `grid 20` shows a dot every 20 of the canvas's units over the whole view, behind the shapes: after the drawing's `clear`, first when it has none, in the theme's `border` color. Zoomed out, dots closer than 8 pixels are thinned to every second, fourth and so on. `snap` (with a grid) says that what is moved lands on the grid; the pointer itself stays free, and code puts the moved thing on the grid with `snapped(value, step)`, the multiple of step nearest to value (a half rounds up; a step of 0 gives the value): `boxX = snapped(sheet.x - grabX, sheet.grid)`. Code reads and sets both while the app runs: `sheet.grid` (0 for none) and `sheet.snap`, as the line says until a handler sets them, so an app can offer Show grid and Snap to grid.
- `background panel` (a theme color by name, or written out: `background "#f5f0e1"`) is what `clear()` without a color fills the canvas with; without it, the theme's `panel`. Code reads and sets it as `canvas.background`, as the line says until a handler sets it; an empty one is the theme's panel again. A person's chosen color can so reach a canvas through a setting: `sheet.background = settings.diagramColor`.
- `dots 3 muted` says how many pixels across a grid's dots are (1 to 20; 2 without it) and their color (a theme color by name, or written out; the theme's `border` without it, and the color can be left out: `dots 3`). They are the same size whatever the zoom. Code reads and sets them as `canvas.dotSize` and `canvas.dotColor`, as the line says until a handler sets them; an empty color is the theme's border again.
- `on bend: handler` lets people bend the canvas's links, as Clarity's diagram designer does: a press on a link adds a point there (or takes the point under it), dragging moves that point, and a double click on a point takes it away; the points of the link in hand show while it is. When the person lets go, the handler runs and reads which link (`canvas.bent`, its name) and its points now (`canvas.via`, a `list<integer>`), to keep them: `association.via = sheet.via` and `save association`. The canvas draws the link with those points until the drawing comes back with them. A press on a shape drawn over a link (a box above its line) is not a bend, and a press on no link is a click or a drag as before.

```xml
<!-- sheets/design/Sheet.cxaml -->
<Window class="sheets.Sheet">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="SheetView" />
  </Content>
</Window>
```

```xml
<!-- sheets/design/SheetView.cxaml -->
<View class="sheets.SheetView">
  <Data name="boxX" type="integer" default="20" />
  <Data name="boxY" type="integer" default="20" />
  <Data name="startX" type="integer" />
  <Data name="startY" type="integer" />
  <Canvas name="sheet" at="1,1" span="12" rows="10" units="{&quot;height&quot;:2000,&quot;width&quot;:3000}" zoom="true" none="surface" ondraw="drawSheet" ondrag="dragged" ondrop="dropped" />
</View>
```

```clarity
code SheetView
    handler drawSheet
        sheet.clear("panel")
        sheet.rect(boxX, boxY, 200, 100, "raised")

    handler dragged
        if startX == 0
            startX = boxX
            startY = boxY
        boxX = startX + sheet.x - sheet.fromX
        boxY = startY + sheet.y - sheet.fromY

    handler dropped
        startX = 0
        startY = 0
```

- `on connect: handler` lets people draw a new line from one box to another, as Clarity's diagram designer does. A box is a shape the canvas can hit (a rect, circle, ellipse, ring, picture or object), together with the shapes under it that hold it whole, so a box drawn as an outline, a body and a header is one box; a shape over the whole paper is the paper. While the pointer is over a box, a small dot in the theme's accent shows in the middle of each side, with a crosshair over a dot. A press on a dot starts a new line (before the box's drag and before a bend): a dashed line follows the pointer and the box under it is outlined. No handler runs while it is drawn. When the person lets go, the handler runs once and reads where the line started (`canvas.fromX`, `canvas.fromY`: the middle of the box it started from) and where it was let go (`canvas.x`, `canvas.y`), in the canvas's units as a drop reads them, so the code finds the two boxes the way its drag finds the one it moves. Let go on the same box or on empty paper, it still runs and the code decides; let go where it was pressed, nothing runs. `canvas.connecting` (a bool, false until code sets it) makes any press on a box start a new line instead of a drag, from where it was pressed, so an app can offer a line tool as the designer's palette does. The dots and the line are drawn over the drawing, never in it, and look the same on every surface. A new line is one edit for undo, like a drop. The right button draws a new line too, from anywhere on a box, without the line tool: a right press and drag from one box to another is the same new line, and a right click that does not move opens the canvas's `menu` instead. Only a canvas whose code draws it has `on connect`: the checker refuses it on a canvas with a `mode` or one an engine draws, and without `on connect` a canvas shows no dots and its pointer works as before.

```xml
<!-- modelling/design/Diagram.cxaml -->
<Window class="modelling.Diagram">
  <Content name="main" at="1,1" span="12" rows="11">
    <View name="DiagramView" />
  </Content>
</Window>
```

```xml
<!-- modelling/design/DiagramView.cxaml -->
<View class="modelling.DiagramView">
  <Data name="types" type="list&lt;ClassType&gt;" />
  <Canvas name="sheet" at="1,1" span="12" rows="10" units="{&quot;height&quot;:3000,&quot;width&quot;:4000}" zoom="true" none="surface" ondraw="drawTypes" onconnect="typesConnected" />
  <Button name="lineTool" at="1,11" span="2" caption="@lineToolCaption" onclick="lineToolClicked" />
</View>
```

```clarity
code DiagramView
    handler typesConnected
        let sourceId = 0
        let targetId = 0
        for t in types
            if sheet.fromX >= t.x and sheet.fromX <= t.x + 200 and sheet.fromY >= t.y and sheet.fromY <= t.y + 100
                sourceId = t.id
            if sheet.x >= t.x and sheet.x <= t.x + 200 and sheet.y >= t.y and sheet.y <= t.y + 100
                targetId = t.id
        if sourceId != 0 and targetId != 0 and sourceId != targetId
            message "From {sourceId} to {targetId}"

    handler lineToolClicked
        sheet.connecting = not sheet.connecting
```

- `on resize: handler` lets people resize a box by dragging its corner, as Clarity's diagram designer does. A box is what `on connect` calls a box. While the pointer is over a box, a small square grip in the theme's accent shows at its bottom-right corner, with the resize cursor over it. A press on the grip (it comes before the box's dot, carry, bend and pan) starts a resize: a dashed outline of the box at its new size follows the pointer. The box's top-left corner stays; its right and bottom edges follow the pointer in whole units, on the grid when the canvas has `snap`, and it is never smaller than 40 by 30 units (on the grid, the next grid step past that). No handler runs while it is dragged. When the person lets go, the handler runs once and reads where the box's bottom-right corner was (`canvas.fromX`, `canvas.fromY`) and where it is now (`canvas.x`, `canvas.y`), in the canvas's units. The code finds the box whose corner was at `fromX`, `fromY` (a point just inside it, `fromX - 1`, `fromY - 1`, is in the box) and makes it `canvas.x - box.x` wide and `canvas.y - box.y` high. Let go at the size it had, nothing runs; a pointer taken away mid-drag drops it. A resize is one edit for undo, like a drop. `canvas.resizable` (a bool, true until code sets it) turns the grip off and on, so an app shows it only while its boxes have a size of their own. The grip and the outline are drawn over the drawing, never in it. Only a canvas whose code draws it has `on resize`: the checker refuses it on a canvas with a `mode` or one an engine draws, and without `on resize` a canvas shows no grip and its pointer works as before.

```clarity
code Diagram
    handler typesResized
        for t in types
            if sheet.fromX - 1 >= t.x and sheet.fromX - 1 <= t.x + t.width and sheet.fromY - 1 >= t.y and sheet.fromY - 1 <= t.y + t.height
                t.width = sheet.x - t.x
                t.height = sheet.y - t.y
                save t
```

- `sin(degrees)` and `cos(degrees)` give a decimal, rounded to 10 decimals.

### Canvas modes

A canvas the person changes has a `mode`; without one, its draw handler draws it and code handles its pointer. Every mode shares the same parameters, the same event and the same property, and each change the person makes is one edit, which Ctrl+Z takes back (see [Undo](#undo)). The runtime does what the gesture says: it snaps to the grid when `snap` is on, keeps the change in the data, and chooses what was touched.

| Mode | Binds | What the person does |
|---|---|---|
| `mode objects types at x, y` | nothing: its objects are the records of the list `types`, placed by their integer fields `x` and `y` | drags an object (the box its draw handler marks with `canvas.object`) and lets it go; the record's `x` and `y` move there |
| `mode paint` | an `image` field: `bind note.picture` | paints with the pen or rubs out with the eraser; each stroke is the picture after it |
| `mode vector` | a `text` field, which holds the drawing as SVG: `bind note.drawing` | draws lines, rectangles, ellipses, paths and texts with the tools of its toolbar; with `select` drags a shape, and Delete deletes it |

- `tools pen, eraser` names the tools of the mode the person may use, in the order its toolbar shows them: `pen` (soft edges), `pencil` (hard pixels), `eraser`, `fill` (the area of near colours under the press takes the ink) and `picker` (the pixel pressed becomes the ink) for paint, `select`, `line`, `rect`, `ellipse`, `path` and `text` for vector, `select` (drag an object) and `connect` (draw a new line from one object to another, which runs `on connect`) for a diagram in mode objects with `on connect`; all of them without it. A diagram without `on connect` has no tools: a press drags.
- `toolbox` puts a canvas's tools in a toolbox (a diagram's built-in one: select, connect, zoom in and out), as a paint program's: a grid of the tools with the brush's size and opacity and the foreground and background colours (a click on one opens the palette; the arrow swaps them). `toolbox` (or `toolbox popup`) floats over the window, dragged by its title and folded with its arrow; `toolbox slideout` lies against the canvas's right edge, as high as the canvas. Without it the tools are a strip on the canvas. The toolbox's tool, size and foreground colour are the canvas's `tool`, `brush` and `ink`, which code reads and sets too.
- `toolboxPage="CoverTools"` shows a toolbox page instead of Clarity's own: a toolbox is designed as a view is, in its own file with a `<Toolbox class="novel.CoverTools">` root, its parts on its grid (`columns="4"`), and placed in the canvas that names it, its code joining the window's. Its parts have a given look: `tool` (a button with an icon), `toolgroup` (tools inside it, one button showing the tool last used, the others beside it while it is held or right-clicked), `toolsize` and `toolopacity` (the brush's size and the paint's opacity, less and more), `toolcolors` (the foreground and background colours, picked and swapped), and buttons, labels, headings and lines. What a tool does is built in, `does pencil`, or coded, `on click: stampStar`, a handler of the toolbox's code; the built-in actions are the canvas tools of paint and vector (`pen`, `pencil`, `spray`, `eraser`, `fill`, `picker`, `select`, `ellipseSelect`, `lasso`, `line`, `rect`, `ellipse`, `path`, `text`) and `rotate`, `flipAcross`, `flipDown`, `crop`, `import`, `export`, `zoomIn` and `zoomOut`. A tool the canvas's mode does not have is an error at the canvas, so one toolbox serves a painting canvas, another a vector drawing, and coded tools any other use. New window offers a toolbox to start from. On a painting canvas, `does adjust` opens the colour dialog (hue, saturation, brightness, contrast), shown on the picture as they change and kept with OK as one edit, on the selection or the whole picture; `toollayers` lists the picture's layers while the canvas is open: each shown or hidden, taken in hand to paint on, added, removed and moved, the picture kept being the shown layers laid over each other (a change of the picture otherwise, Ctrl+Z or another record, folds them back into one).
- `brush 4` (1 to 100 pixels, 2 without it) and `ink accent` (a theme color by name, or written out) are how wide and in which color paint and vector draw. A vector drawing keeps a theme color written out, so other programs show it alike.
- `on change: handler` runs when an edit of the canvas ended: an object dropped, a stroke painted, a shape added, changed, moved or deleted. Without it the change is lost, which the designer warns about.
- `canvas.selected` is what the person touched last: the record of the object (mode objects), or the place of the shape in the drawing, from 1, and 0 for none (mode vector). Handlers set `canvas.tool`, `canvas.brush` and `canvas.ink`, so buttons can be a palette.
- The SVG of a vector drawing has one element on a line. A drawing made in another program shows too; a line Clarity does not read as one of its shapes is kept as it is, and shows, but does not move.

```clarity
entity ClassType
    name: text
    x: integer
    y: integer

entity Note
    picture: image
    drawing: text
```

```xml
<!-- notes/design/Diagram.cxaml -->
<Window class="notes.Diagram">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="DiagramView" />
  </Content>
</Window>
```

```xml
<!-- notes/design/DiagramView.cxaml -->
<View class="notes.DiagramView">
  <Data name="types" type="list&lt;ClassType&gt;" />
  <Canvas name="diagram" at="1,1" span="12" rows="10" mode="objects" objects="{&quot;list&quot;:&quot;types&quot;,&quot;x&quot;:&quot;x&quot;,&quot;y&quot;:&quot;y&quot;}" units="{&quot;height&quot;:3000,&quot;width&quot;:4000}" zoom="true" grid="20" snap="true" none="surface" ondraw="drawTypes" onchange="typeMoved" />
</View>
```

```xml
<!-- notes/design/Sketch.cxaml -->
<Window class="notes.Sketch">
  <Content name="main" at="1,1" span="12" rows="9">
    <View name="SketchView" />
  </Content>
</Window>
```

```xml
<!-- notes/design/SketchView.cxaml -->
<View class="notes.SketchView">
  <Data name="note" type="Note" />
  <Canvas name="sketch" at="1,1" span="6" rows="8" bind="note.picture" mode="paint" brush="4" ink="accent" none="surface" onchange="saveNote" />
  <Canvas name="drawing" at="7,1" span="6" rows="8" bind="note.drawing" mode="vector" grid="10" snap="true" none="surface" onchange="saveNote" />
  <Button name="redButton" at="1,9" span="2" caption="@redCaption" onclick="red" />
</View>
```

```clarity
code DiagramView
    handler drawTypes
        diagram.clear()
        for t in types
            diagram.object(t, t.x, t.y, 260, 120)
            diagram.rect(t.x, t.y, 260, 120, "raised")
            diagram.text(t.x + 130, t.y + 14, t.name)

    handler typeMoved
        save diagram.selected

code SketchView
    handler saveNote
        save note

    handler red
        sketch.ink = "#ff0000"
```

### Data events

An entity's code block holds its data events: code that runs whenever one of its records is saved or deleted, from any window, action or other event. The block has the entity's name and is in the same file as the entity.

```clarity
code Order
    before save
        if order.status == Approved and not withinCredit(order)
            refuse @orderOverCreditLimit(number = order.number)

    after save
        log_change("saved " + order.number)

    before delete
        if order.status != Pending
            refuse @onlyPendingDeletable
```

| Event | Runs |
|---|---|
| `before save` | before a record is stored; what it changes in the record is stored |
| `after save` | after it is stored; a new record has its id by then |
| `before delete` | before a record is removed |
| `after delete` | after it is removed |

- **The record** is named after the entity in camelCase: `order` in the events of `Order`, `orderLine` in those of `OrderLine`. A new record has id 0 in `before save`.
- **An event is an action** on its record. It can read and change records, `save` and `delete` others, and call functions and actions; it declares no effects. Like an action, it does not talk to the screen: no `message`, `open`, `close` or part properties.
- **`refuse "text"`** stops the save or delete. The text is the error the window shows, so a refusal reads as the message for the person using the application.
- **All or nothing.** The save or delete runs in one transaction with its events, and with the saves and deletes they make in the same database. When an event fails or refuses, none of it is kept, and a new record is still new.
- An event that saves its own record in a save event, or deletes it in a delete event, would run itself again without end; the checker reports it. Each event is written once per entity, and an entity's code block has no handlers.
- Renaming an entity renames its code block and the record's name in its events.

## 9. Functions and actions

Logic that is not tied to one window goes into functions and actions in a module. Handlers, functions and actions can call them.

```clarity
function withinCredit(order: Order) -> bool
    return order.total <= order.customer.creditLimit

function orderTotal(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total

action approve(order: Order)
    require order.status == Pending
    require withinCredit(order)
    effects db.write
    order.status = Approved
    save order
```

- **The signature** is `name(param: type, ...)`, optionally followed by `-> type` for the result. The types are `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an entity (one record), an enum, and `list<Entity>`.
- **A default** makes a parameter optional: `limit: integer = 0`. It is a value written out (a number, a text, `true` or `false`, `none` or an enum's value), and the parameters with one come after those without. A call leaves them out, or gives any of them by name after the values in order: `list_templates("published")`, `list_templates("published", limit: 10)`, `greeting(times: 2, name: "Bo")`. Each parameter is given once, and every one without a default is given. Handlers, built-in calls and an engine's functions take their values in order, without names.
- **`allow when expr`** on an action says who may run it, reading `me` (see [Signing in](#signing-in)); it comes before the statements.
- **`require expr`** checks the arguments before the body runs. **`ensure expr`** checks the result after it, as `result`. Both must be bool, and both come before the statements.
- **`effects`** lists what an action does to data and the world: `db.read`, `db.write`, `files.read`, `files.write` and `net`, comma separated. An action that uses `save` or `delete`, or that calls an action with `db.write`, must declare `db.write`; one that lists or reads files declares `files.read`, one that writes a file `files.write`, and one that calls a web API `net`. A function cannot reach files or the network at all; handlers can, as they can save.
- **A function** has no effects. It cannot save or delete, call actions, show messages or open windows. That makes it safe to call anywhere, including in conditions.
- **An action** can change data. Neither functions nor actions talk to the screen: `message`, `open`, `close` and part properties belong in handlers.
- A function or action with a result returns a value on every path; the checker reports a path that does not.

### Constants

A constant is a value written out once and read by name, never set:

```clarity
const lives = 3
const speed: decimal = 1.5
const levels = [1, 2, 3]
const pillRed: picture = "pill-red.svg"
```

- **Where:** a `const` line in a module, read from its code, its windows' handlers and its engines; or inside an engine written in Clarity, read by that engine. An engine's own constant comes before its module's.
- **What:** a value written out: a number, text, yes or no, a key (`@welcome`), an enum's value, or a list of them. Its type is the value's own, or the one written after the name; a whole number in a `decimal` constant is a decimal.
- **`picture`** is text naming a picture of the resources folder; the checker finds the file there, once, where the constant is declared.
- Setting a constant is an error. Generation writes each constant's value where it is used, so every stack and every engine has it alike.

### Engines

An engine is a processing module that every stack runs the same way: pricing, scheduling, a solver, the logic of a game. It is written in Clarity (see [An engine written in Clarity](#an-engine-written-in-clarity)) or compiled to WebAssembly from another language. A module declares a WebAssembly engine with its file and the functions it has; code calls those functions like its own.

```clarity
engine Pricing from "pricing.wasm"
    uses clock, random
    says @outOfStock
    limit 2 seconds, 64 MB
    state per app
    on open
    function total(lines: list<Line>) -> money
    function sizeOf(count: integer) -> Size
    function discounted(order: Order) -> Order
```

```clarity
function orderTotal(order: Order) -> money
    return Pricing.total(order.lines)
```

- **The file** is in the `engines` folder of the module folder (`shop/engines/pricing.wasm`). Every generated application carries it inside the program.
- **Each function** has a signature like a function's, and always a result: `-> type`. Parameters and results are values (`text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`), `bytes`, enums, records and `list<Entity>`; not `file` or `list<file>`. An engine that works on a document gets its bytes: `Reader.pages(read bytes of f)`.
- **A call** is `Engine.function(args)`. The checker types it against the declaration, as it does a call to a function. Handlers, functions and actions can all call an engine: it has no effects.
- **The engine computes; the application does all input and output.** An engine reaches no database, file, network or screen. Code reads the records, hands them in, and saves what comes back: a record an engine returns is a copy, saved only when code saves it.
- **`uses`** names the services of Clarity the engine may call, comma separated: `clock` (the time), `random` (a random number), `log` (a line in the application's log) and `language` (the language the person picked, such as `nl`). Each is a function the file imports from module `clarity`; a file that imports anything else, or a service the declaration does not name, is refused. Without `uses`, an engine imports nothing.
- **`says`** lists the keys of the texts the engine gives people to read, comma separated: `says @youWin, @aDraw`. The engine returns a key (`"youWin"`), whose text the language files give in the person's language; an engine's refusal by a key shows the same way. A text an engine gives that is no key shows as it is.
- **`limit`** says what one call may take: `limit 2 seconds`, `limit 64 MB` or both, `limit 2 seconds, 64 MB`. A call that runs longer is stopped, and fails with an error that says so; the engine never gets more memory than its limit. Without a limit line, a call may take 5 seconds and the engine 256 MB. A time limit is between 1 and 3600 seconds, a memory limit between 1 and 4096 MB.
- **State.** Without a state line, each call starts the engine afresh, so nothing carries over from one call to the next. `state per app` keeps one instance for the whole application: what the engine holds in its memory carries from call to call, such as a loaded dictionary. `state per window` keeps an instance for each open window, made at its first call there and dropped when the window closes: a game's board, a document being edited. Only a window's handlers call an engine with state per window, each in its own window. A call that fails or runs past its limit drops the instance, since its memory may be half changed; the next call starts a new one. A refusal keeps it.
- **`on open`** runs the engine's own `clarity_open` whenever an instance is made: once per instance with state, before every call without. There is no `on close`: an engine cannot save or show anything, so a window that must keep what an engine holds asks it in its own `on close` handler and saves the answer.
- **Checked twice.** The designer reads the file and reports a file that is not in the folder, is no WebAssembly, imports what the engine does not use, or lacks a declared function. The application checks it again when it starts, and stops there rather than at the first call.
- **An error** that the engine gives back (its own refusal) reaches code as a refusal, in the engine's words. An engine that fails while it runs is an error that names the engine.
- The declaration is visible to the modules that `use` the module, like its functions. Two engines of one module have different names, and an engine has each function once.

#### An engine beside the screen

An engine declared `beside the screen` draws a canvas frame by frame, where the canvas is drawn; a canvas `runs` it in place of a draw handler.

```clarity
engine Screen from "screen.wasm"
    beside the screen
    on open
    limit 1 second
    tells finished(time: integer)
    hears start(level: integer)
```

```xml
<!-- game/design/Play.cxaml -->
<Window class="game.Play">
  <Content name="main" at="1,1" span="12" rows="11">
    <View name="PlayView" />
  </Content>
</Window>
```

```xml
<!-- game/design/PlayView.cxaml -->
<View class="game.PlayView">
  <Data name="best" type="integer" />
  <Canvas name="sky" at="1,1" span="12" rows="10" units="{&quot;height&quot;:100,&quot;width&quot;:160}" runs="Screen" tells="[{&quot;handler&quot;:&quot;gameOver&quot;,&quot;tell&quot;:&quot;finished&quot;}]" none="surface" />
  <Button name="go" at="1,11" span="2" caption="@startCaption" onclick="startGame" />
</View>
```

```clarity
code PlayView
    handler gameOver(time: integer)
        best = time

    handler startGame
        sky.start(2)
```

- **Each canvas that runs it** has an instance of its own, from when its window opens until it closes. Code does not call it: it has no `function` lines and no `state` line.
- **Every frame** calls the file's `clarity_tick` with the milliseconds since the last frame (0 first), then `clarity_draw`, whose value is the canvas's shapes in its units, as a draw handler sends them (`{"op": "circle", "x": 50, "y": 50, "r": 5, "color": "accent"}`). A frame is skipped while the last is busy.
- **Pointer and keys** on the canvas go to the file's `clarity_pointer` (`[kind, x, y]`, kind `down`, `move` or `up`, in the canvas's units rounded down) and `clarity_key` (`[kind, key]`, kind `down` or `up`), when it exports them. They wait for the next frame and go first, before its tick. While the pointer is down the canvas keeps it, so beside the drawing it is at the nearest edge. A key goes down once however long it repeats; it is named as in `conformance/screen-engines.json` (`a`, `7`, `Space`, `ArrowLeft`, `Shift`, `F1`). A canvas whose engine takes keys can be clicked or tabbed to for the keyboard; a key with Control, Alt or the command key stays the window's shortcut.
- **Tells and hears.** `tells finished(time: integer)` is what the engine tells its window: the canvas wires it to a handler as a custom part's tell (`canvas ... runs Screen on finished: gameOver`), which runs with the tell's values (`handler gameOver(time: integer)`). The file tells it in the answer of any call: `{"value": null, "tells": [{"tell": "finished", "values": [1200]}]}`. `hears start(level: integer)` is what a handler tells it through the canvas, `sky.start(2)`; the file has a function `start`, which runs with `[2]` at the start of the next frame. Values are plain: text, numbers, dates, durations, enums, bool. Only an engine beside the screen tells and hears.
- **A frame that fails** or runs past the limit ends the engine; the canvas shows why (`screenEngineFailed`, with the host's message in `{problem}`).
- **The checker** wants a `runs` to name an engine beside the screen, no draw handler on that canvas, a file that exports `clarity_tick`, `clarity_draw` and a function per hear (or, written in Clarity, functions of those names, below), tells wired to tells the engine has, with handlers that take their values; an engine beside the screen that no canvas runs is a warning.

#### An engine written in Clarity

An engine without `from` is written in Clarity: what it keeps between calls, and its functions with their code. Clarity compiles it to a WebAssembly file when it builds the application, so every stack runs the same file and it needs no other language.

```clarity
engine Board
    uses random
    state per window
    keep squares: list<integer> = [0, 0, 0, 0, 0, 0, 0, 0, 0]
    keep moves: integer = 0

    function play(square: integer) -> text
        if squares[square] != 0
            refuse @thatSquareIsTaken
        squares[square] = 1
        moves = moves + 1
        return @yourMove

    function mark(square: integer) -> integer
        return squares[square]
```

- **`keep name: type = value`** is a value the engine keeps from call to call, with what it starts as. Its functions read and set it by name. A keep is a plain value or a list of them (`list<integer>`); the value it starts with is written out.
- **`state`** works as for any engine: none gives every call fresh values, `state per app` one set for the whole application, `state per window` one set for each open window, dropped when the window closes.
- **A call changes nothing when it fails.** A function works on a copy of what the engine keeps, which it keeps only when the call succeeds; a refusal or an error leaves the values as they were.
- **Its functions** are written as functions are, with `require` and `ensure` if wanted, and call each other by name (`answer()`). Code outside calls them as `Board.play(5)`.
- **It keeps to itself.** An engine has no window, data, parts or database: `message`, `open`, `save`, `find` and the like are errors in it. It returns what the screen should show, and its texts are keys, translated like the project's own.
- **`uses progress`** gives it `progress(done, total)`, on a line of its own: how far a long call is, such as `progress(day, total)` in a loop over the days of a year. While the handler that made the call runs, its window shows a bar of how far it is with a Stop button (Clarity's word `stopCall`), keeps painting and takes no other click. Stop ends the call, which fails with `it was stopped`, shown as any error. A page a server serves to a browser shows no bar. An engine beside the screen draws as it goes and has no progress.
- **`uses random`** gives it `random(n)`: a whole number from 1 to `n`. No other service is needed in Clarity: the language the person picked is already how its texts show.

- **Beside the screen** too: an engine written in Clarity has no words of its own for frames. Its frames are functions of the interface's names, what it draws is a list of records it returns, a hear is a function of its name, and `tell finished(time)` puts a tell in the answer of the call it is in:

```clarity
record Shape
    op: text
    x: integer
    y: integer
    r: integer
    color: text, default "accent"

engine Sky
    beside the screen
    tells finished(time: integer)
    hears start(level: integer)
    keep elapsed: integer = 0
    keep stage: integer = 1

    function clarity_tick(ms: integer)
        elapsed = elapsed + ms
        if elapsed > 10000
            tell finished(elapsed)

    function clarity_draw() -> list<Shape>
        let dot = new Shape
        dot.op = "circle"
        dot.x = stage * 10
        dot.y = 50
        dot.r = 5
        return [dot]

    function start(level: integer)
        elapsed = 0
        stage = level
```

  The checker wants `clarity_tick(ms: integer)`, `clarity_draw()` giving a list of records, and a function for each hear with its values; `clarity_pointer(kind: text, x: integer, y: integer)` and `clarity_key(kind: text, key: text)` are there when the engine wants the pointer and keys. What it keeps starts as written, so it has no `on open`. A record's fields that a shape does not have are left alone by the canvas.

#### Records

A record is a value with fields that belongs to no table: declared like an entity, with `record`, never saved or found.

```clarity
record Dot
    x: integer
    y: integer
    size: decimal, default 1.5
    tags: list<text>
```

- **`new Dot`** has every field at its default: the one written with `default`, or 0, empty text, no, an empty list. A record's decimal needs no precision and scale, since it has no column.
- **`d.x`** reads a field and **`d.x = 3`** sets it. A record is a value: setting a field changes that record only, as an item of a list does.
- **Copied, never shared.** Putting a record into a list, giving it to a variable, a window's data or a function, or reading it from a list or from `grid.selected` makes a copy. After `let d = new Dot`, `dots = dots + [d]` and `d.x = 3`, the dot in the list still has its own `x`. Every stack behaves alike: Rust, Go, C and C++ copy the struct, C# copies the object (`d.Copy()`).
- **Lists of records** (`list<Dot>`, `[a, b]`) are read by item and by `for`, and grow by a list of the same records: `dots + [d]`. An empty one names its type: `let dots: list<Dot> = []`.
- **Fields** are values: text, numbers, yes or no, dates, enums and lists of them, or a reference to a stored record (`project: ref Project`), kept as its id; not pictures or files, and no reference to another record. An engine uses records of values only.
- **A record is never none.** It always has its fields; compare a field instead. Only a part's selected row is set to `none`.
- **Never stored.** `save`, `delete` and `find` on a record are errors.
- **Engines** written in Clarity give and take records as JSON objects of their fields.
- **App code** uses them too: functions and actions take and give them, a window holds one (`<Data name="picked" type="Node" />`, new when the window opens) or a list of them (`type="list&lt;Node&gt;"`, empty when it opens), and a `list`, `table`, `tree` or `treetable` shows such a list as it shows records of an entity: `columns` and `show` name its fields, `grid.selected` is the record of the chosen row, and `on select` and `on open` run. A record has no id, so a row is its place in the list. Giving the window a new list (`nodes = outline()`) selects no row; code selects one again with `nav.selected = ...`. A table of records is not edited by the person (`edit`) and its rows are not moved by hand (`reorder`): build the list in code.

```clarity
record Node
    key: text
    parent: text
    label: text
    project: ref Project

function outline(projects: list<Project>) -> list<Node>
    let all: list<Node> = []
    let top = new Node
    top.key = "land"
    top.label = "Landscape"
    all = all + [top]
    for p in projects
        let n = new Node
        n.key = p.name
        n.parent = "land"
        n.label = p.name
        n.project = p
        all = all + [n]
    return all
```

```xml
<!-- modelling/design/Workspace.cxaml -->
<Window class="modelling.Workspace" onopen="loadNodes">
  <Content name="main" at="1,1" span="12" rows="20">
    <View name="WorkspaceView" />
  </Content>
</Window>
```

```xml
<!-- modelling/design/WorkspaceView.cxaml -->
<View class="modelling.WorkspaceView">
  <Data name="projects" type="list&lt;Project&gt;" />
  <Data name="nodes" type="list&lt;Node&gt;" />
  <Tree name="nav" at="1,1" span="4" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" onselect="chosen" />
</View>
```

```clarity
code Workspace
    handler loadNodes
        projects = find p in Project order by p.name
        nodes = outline(projects)
        nav.selected = nodes[1]

code WorkspaceView
    handler chosen
        if nav.selected.project != none
            message nav.selected.project.name
```

How to build an engine in another language, and what it exports, is in the manual: [Engines](manual/code.engines.md).

### Commands

A project with the `cli` target is also a program without a window: commands typed in a terminal, over the same records, actions and settings as its windows. One `cli` block, named after the project, gives the program's description; each command is declared in any module, its statements below it as a function's are:

```clarity
cli Shop
    help @shopHelp

command customer add(name: text, email: text)
    help @addCustomerHelp
    option vip: bool, help @vipHelp
    option limit: money, default 1000, min 0, max 5000, help @limitHelp
    let c = new Customer
    c.name = name
    c.email = email
    c.creditLimit = limit
    save c
    print @customerAdded(name = c.name)

command customer remove(name: text)
    help @removeCustomerHelp
    let found = find c in Customer where c.name == name
    if count(found) == 0
        refuse @noSuchCustomer(name = name)
    if ask @removeIt(name = name) yes @remove no @keep
        delete found[1]
```

    $ shop customer add Ada ada@example.org --vip --limit 500
    $ shop customer remove Ada
    $ shop help customer add

- **The program** is named after the project, in lower case with hyphens (`MyShop` is `my-shop`). Its help comes from the `cli` block, each command's and option's from its `help`; a command or option without one is a warning.
- **A command's name** is one word or a group and a word: `command customer add` is `shop customer add`. A name in camelCase is typed with hyphens (`addAll` is `add-all`). A group is there when a command names it, and cannot be a command of its own.
- **Arguments** are the parameters, given in order and all required. **Options** are `option name: type` lines before the statements, with `default`, `min`, `max`, `required` and `help`, given as `--name value` or `--name=value` in any order; a `bool` option is a flag (`--vip`). An option left out has its default, or the empty value of its type. Both are text, a number, money, a date, a moment, a duration or an enum (typed by its value's name); a `bool` is only an option, and a record is no argument: look it up with `find`.
- **On the command line** numbers are written with a dot and no groups, dates as `2026-10-23`, moments as `2026-10-23T14:30`, durations as `3:45`, `1:02:03` or seconds; `--` ends the options. A wrong command line (an unknown command or option, a missing value, a value of the wrong kind or past `min` or `max`) writes what is wrong and the command's usage, and ends with exit code 2. `--help` anywhere, `help` before a command, or a group alone writes the help.
- **What a command sees:** its arguments and options by name, `settings` (the ones the windows keep, read and set; a change is kept), and the functions, actions, engines, queries, `save`, `delete` and files a handler has. It has no window: `open`, `close`, `reload` and part properties are errors in it.
- **`print`** writes a line to standard output: a text, with slots, or a value. `print list columns a, b` writes the records of a list as a table: the fields' labels as headings, the columns lined up, numbers to the right.
- **`message`** writes a line to standard error.
- **`ask`** gives its answer in a command: `if ask @sure yes @go no @stop`, `let n = ask @howMany for integer default 3`, `let k = ask @which for CustomerKind`, `let c = ask @who for Customer from found`. It waits on the terminal; an empty answer takes the default, and without a terminal (piped input, a script) a question with a default takes it and one without ends the program with exit code 2.
- **`exit n`** ends the program with exit code `n`, 0 to 255. A command that finishes ends with 0; a `refuse` writes its text to standard error and ends with 1.
- Clarity's own words of the command line (the help's headings, what it says of a wrong command line) are keys too, as the screen's are; a project with commands gives them their texts.
- **`agent`**, a line before the statements, offers the command to AI agents as a tool of the app's MCP service (docs/ledger/app-mcp.md). It is off without the line. A command for agents needs its `help`, which is how an agent picks it, and agents call it by its words joined with hyphens (`customer-add`). Clarity works out what a command changes (it saves or deletes records, writes a file, sets a setting, or sends something out with an HTTP call that is no `get`, itself or through an action with those effects), so the person can be asked before the change.
- **`<program> mcp`** serves the commands for agents over MCP on standard input and output: an agent starts `shop mcp` as one of its MCP servers. A call runs the command as if it were typed, with the call's values as its arguments and options; what it prints is the answer, and a refusal, an exit code other than 0 or a wrong value is an error the agent reads. A question the command asks (`ask`) goes to the agent; when its client cannot answer, a yes or no is a no, a value takes its default, and without one the command stops. While the app is closed nobody is there to allow a change: the app's settings `agentReadsWhileClosed` and `agentChangesWhileClosed` (see [Settings](#settings)) allow reads and changes, else the call is refused in Clarity's words `agentClosed`, "{app} is not open; open it to allow this.". A setting the person never set is what the project file says (`agents while closed read change`), and off without the line. A command for agents needs no `cli` target: the windowed program serves `<program> mcp` itself. `mcp` is the program's own word: no command or group is named `mcp`.
- **While the app is open**, `<program> mcp` hands each call to it, so the person is there: the first read of a connection and every change ask in Clarity's dialog (Allow once, Allow for this session, Refuse), a command's `ask` goes to the agent, and the call runs on the open app's data. A change is one step: all of it is kept, or none when it is refused; the open window reads its data again, and a note in it says what the agent did, with Undo, until the person closes it. Several apps run side by side: each answers only its own agents.
- **The person's say:** the switches for while the app is closed are settings of the app, so they show in its settings window with its other settings; the developer sets up everything else in the IDE's Agents part.

```clarity
command customer add(name: text)
    help @addCustomerHelp
    agent
    let c = new Customer
    c.name = name
    save c
```

- **The guidance for agents** is the project's `agent/guidance.md`: Markdown in four sections under fixed headings (`What the app is for`, `The app's words`, `How work flows`, `Rules the app does not check`), written once in the project's language. Agents get it when they connect, followed by the app's data summed up. It says what the declarations cannot: what the app is for, what its words mean, the order work goes in, rules no check enforces. Agents write it a section at a time with Clarity's tools `guidance` and `set_guidance`, never as a whole file. A name in backticks must be one of the app's (a command as typed, an entity, `Entity.field`, an enum or its value); the checker warns of one that is not, and of an app with commands for agents and no guidance.

## 10. Statements and expressions

### Statements

One statement per line; `if`, `else` and `for` take an indented block.

| Statement | Meaning | Where |
|---|---|---|
| `let name = expr` | A local, with the type of the expression. | everywhere |
| `let name: type = expr` | A local with a declared type. | everywhere |
| `target = expr` | Sets a local, a data field (`customer.creditLimit`), an item of a list (`squares[3] = 1`) or a part property (`save.enabled`). | everywhere; part properties in handlers |
| `if expr` / `else if expr` / `else` | Runs a block when the condition, which must be bool, is true. | everywhere |
| `for item in list` | Runs the block once per item of a list. `for i in 1 to n` counts from 1 to n, as `1 to n` is that list. | everywhere |
| `name(args)` | Calls a function or action on its own line. | everywhere |
| `handler_name()`, `handler_name(values)` | Runs another handler of the same window or app, with values for its parameters. | handlers |
| `canvas.shape(values)` | Draws a shape on a canvas; see [Drawing on a canvas](#drawing-on-a-canvas). | its draw handler |
| `return` | Leaves the handler or action. | handlers, actions without a result |
| `return expr` | Leaves with a result. | functions and actions with a result |
| `message expr` | Shows a message. `message info`, `message warning` and `message error` set its level; the default is info. An info message is a note that goes away by itself (or by its small x); a warning or an error shows in Clarity's dialog and is closed with OK. In a command it writes a line to standard error. | handlers, commands |
| `copy expr` | Puts the text (or value), as it shows in the person's language and format, on the system's clipboard once the handler has run, for the person to paste anywhere; of several copies, the last counts. On the web the page uses the clipboard of the browser or the desktop window it runs in. Pair it with a message, as `message @copied`, so the person knows. | handlers |
| `ask question ... then handler(values)` | Asks the person in Clarity's dialog when the handler has finished; the answer runs the handler after `then`. See [Asking the person](#asking-the-person). | handlers |
| `open Window` | Opens a window. | handlers |
| `open Window with name = expr, ...` | Opens a window and fills its data by name. | handlers |
| `close` | Closes this window. | handlers |
| `close with expr` | Closes this dialog with a value of the type it gives, for the handler that asked for it. | handlers of a dialog that gives one |
| `save expr` | Stores a record in its data source. A new record gets its id. A record someone saved since it was read is refused with `savedSince` (see [Entities](#entities)). | handlers, actions, data events |
| `save expr anyway` | Stores a record regardless of its version: overwriting on purpose, never guessed. With sign-in, the action that does it can be limited with `allow when`. | handlers, actions, data events |
| `delete expr` | Removes a record from its data source. | handlers, actions, data events |
| `refuse expr` | Stops with this text (or value) as the error; a save or delete it is part of does not happen. See [Data events](#data-events). | everywhere |
| `reload data` | Reads the window's data again: a list loads all its records again, and a record loads again by its id (a new record stays as it is). A table or list showing a list read again has no row selected afterwards, as when the list is given new rows (`orders = find ...`): they are other records. | handlers |
| `reload record` | Reads a record variable (a local or a parameter) again as it is stored now, with its version, as after a refused save. Locals never share a name with the window's data, so the name says which `reload` it is. | handlers, functions, actions, data events, commands |
| `write expr to store "path"` | Writes text as a file of a file store, at a path inside it, making its folders. See [Files](#files). | handlers, actions with `files.write` |
| `print expr` | Writes a line to standard output; `print list columns a, b` writes the records as a table. See [Commands](#commands). | commands |
| `exit expr` | Ends the command-line program with this exit code, 0 to 255. | commands |

### Expressions

| Expression | Meaning |
|---|---|
| `12`, `9.95`, `"text"`, `true`, `false` | literals |
| `@savedName(name = customer.name)` | a text by its key, its slots filled by name; see [Texts by key](#texts-by-key) |
| `"{album.title}.txt"` | text with slots, filled plainly; see [Text with slots](#text-with-slots) |
| `customer`, `customer.name`, `grid.selected.total` | names and paths |
| `[1, 2, 3]`, `["a", "b"]` | a list of values of one type; an empty one needs its type, as in `let squares: list<integer> = []`; `readings + [value]` is a new list with the items of both |
| `squares[3]` | an item of a list, counted from 1; a position outside the list is an error |
| `Business`, `CustomerKind.Business` | enum values |
| `none` | the empty value of a date, a date and time or a reference field: `loan.returned_on == none`, `order.customer = none`. Nothing else is ever none; empty text is `""` |
| `name(arg, ...)` | a call to a function or action; `count(list)` gives the number of items, `join(texts, separator)` the texts of a `list<text>` in order with the separator between each two (`join(names, ", ")`), `round(value, places)` a decimal or money with at most `places` decimals, rounded half away from zero (`round(2.345, 2)` is 2.35), `length(text)` how many characters a text has (`length("Größe")` is 5), so code can size what it draws to its text, `snapped(value, step)` the multiple of a whole number `step` nearest to `value`, a half rounding up (`snapped(29, 20)` is 20; a step of 0 gives the value), as a canvas's grid snaps, `sin(degrees)` and `cos(degrees)` a decimal rounded to 10 decimals, `today()` the date and `now()` the date and time where the application runs ; the text functions `words(text)` (the words, apart by white space), `split(text, separator)` (the pieces, empty ones kept; an empty separator gives the characters), `trim(text)`, `lower(text)`, `upper(text)`, `replace(text, find, with)`, `startsWith(text, prefix)`, `endsWith(text, suffix)`, `slice(text, from, count)` (characters from a position, from 1, clamped to the text) and `indexOf(text, find)` (the position from 1, 0 when it is not there), alike on every stack (conformance/texts.json): `count(words(chapter.body))` counts a chapter's words |
| `(expr)` | grouping |
| `-x` | negation |
| `a * b`, `a / b` | multiplication and division; `/` always gives a decimal |
| `a div b`, `a mod b` | whole-number division: the quotient of two integers rounded toward zero (`7 div 2` is 3, `-7 div 2` is -3), and what is left, with the sign of `a` (`7 mod 3` is 1); both integers. Dividing by zero is an error, said in Clarity's word `divisionByZero`. Elsewhere `div` and `mod` are plain names |
| `a + b`, `a - b` | addition and subtraction; `+` also joins text |
| `a == b`, `a != b`, `a < b`, `a <= b`, `a > b`, `a >= b` | comparison |
| `x in [a, b, c]` | whether `x` equals one of the values |
| `a to b` | the whole numbers from `a` to `b`, both counted, as a `list<integer>`; empty when `b` is below `a`: `for i in 1 to count(names)` |
| `a contains b` | whether text `a` contains text `b`, ignoring case; for a list, whether it holds item `b` |
| `find c in Entity where ... order by ...` | the records that meet a condition; see [Queries](#queries) |
| `ask question yes a no b`, `ask question for type ...` | in a command, the answer typed on the terminal: a bool for yes or no, or a value of the type; see [Commands](#commands) |
| `new Entity` | a new record of the entity, with its defaults; `save` stores it |
| `files f in store where ... order by ...` | the files of a file store that meet a condition; see [Files](#files) |
| `read text of f` | the text a file holds |
| `read bytes of f` | the contents of a file as they are; see [Files](#files) |
| `not a` | negation of a bool |
| `a and b`, `a or b` | both, either |

From tightest to loosest: member access (`.`) and items (`[i]`), unary `-`, `*` and `/`, `+` and `-`, `to`, comparisons, `in`, `contains` and `matches`, `not`, `and`, `or`. In `write text to store "path"`, the `to` belongs to `write`. An engine written in Clarity has no `to` or `join` yet.

### Texts by key

A text people read is a key, and its slots get their values by name, so a translator can move them and each value shows in the person's format:

```clarity
code OrderForm
    handler approveClicked
        approve(order)
        message @orderApproved(number = order.number, total = order.total)
        approve_button.caption = @approvedOn(day = order.ordered_on)
```

```clarity
translation en
    orderApproved "Order {number} of {total} is approved"
    approvedOn "Approved on {day}"
```

- A key is `@` and its name; with slots, `@key(slot = value, ...)` gives each slot of its text a value. Every slot the default language's text has gets one, and no other.
- A value is checked like any expression; a whole record or list is no value to show, so `customer = order.customer` is an error that suggests one of its fields.
- A value shows as its field shows it: money with its currency, a date in the locale and the field's `format`, an enum value by its label. A local or a parameter shows as its type does.
- The slot `{app.version}` is the project's version, which Clarity fills itself: a use gives it no value, and a caption shows it as it is (`versionLabel` with `@welcomeVersion`, `"Version {app.version}"`).
- A keyed text is a text like any other: code keeps it in a local, passes it to a function, returns it, and puts it in a text value.
- Where it goes straight to the screen (a message, a refusal, a caption, an error, a text value a handler sets), the backend sends the key with each value, and the screen shows its text in the person's language and fills the slots in the person's format.
- Anywhere else, such as a function's result, the backend takes the key's text in the language the screen picked and fills its slots plainly.
- Text joined with `+` in a message, a refusal, a caption or an error is a warning that suggests a key with slots, since such text cannot be translated and its values do not follow the locale.

```clarity
function creditNote(order: Order) -> text
    if withinCredit(order)
        return @withinCredit(number = order.number)
    return @overCredit(number = order.number)
```

### Text with slots

A string in code may have slots too: `"{album.title}.txt"`, `{{` and `}}` for braces. It is filled plainly, in no language, for what people do not read: a file's name, a line written to a file, a value to compare. Where people read it, it is a key.

### Queries

`find` reads the records of an entity that meet a condition, in an order. The result is a list, used like any other: assign it to a window's list data, loop over it with `for`, count it, or return it.

```clarity
code Customers
    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text and c.active order by c.name

    handler cardOpened
        orders = find o in Order where o.customer == customer and o.total > 100 order by o.total desc
```

- `find c in Customer` names each record `c`. The name exists only inside the query, so `c.name` is never confused with a part or local that is also called `name`.
- `where` is optional. Without it, `find` reads every record.
- The condition combines, with `and`, `or`, `not` and parentheses, comparisons between a field of the record and a value: `c.name == x`, `c.total > 100`, `c.name contains x`, `c.kind in [Private, Business]`. A bool field stands on its own: `c.active`, `not c.active`. A reference field is compared with a record: `o.customer == customer`. A date or reference field compared with `none` finds the records where it is empty: `l.returnedOn == none`.
- The value side can be anything that does not use the record: a literal, a local, the window's data, a part property.
- A condition can go through references to the fields of the record they refer to: `o.customer.name contains x`, `t.album.artist.country == "UK"`. A record whose reference is empty does not meet such a condition.
- `order by` takes one or more fields, each optionally followed by `desc`, and may go through references too: `order by a.artist.name, a.title`. `order by c.id` is the order the records were stored in, which every record has. Without `order by`, records come in the order they were stored.
- A query reads one database, so it cannot go through a reference to an entity in another data source.
- The database does the work: the condition becomes a query with parameters, so values never become part of the SQL text.

#### Full-text search

A field marked `search` is in its record's full-text index, and `a matches x` finds the records that hold every word of `x`, each as the start of a word, in any of those fields:

```clarity
entity Album
    title: text(120), required, search
    notes: text, search
    tags: list<text>, search

code AlbumBrowse
    handler searchChanged
        albums = find a in Album where a matches searchBox.text order by a.title
```

- Typing `blue no` finds an album titled *Blue Train* with the notes *on Blue Note*, and one tagged `blues` and `north`, but not *Kind of Blue* alone: every word must be there. Case does not count.
- A word is a run of letters and digits; `hard-bop` is the two words `hard` and `bop`. Text without words, such as an empty search box, matches every record.
- `matches` looks in the record of the query itself (`a matches x`), and combines with other conditions: `where a matches x and a.year > 1960`. The entity needs at least one field marked `search`.
- `contains` stays the exact test: `a.title contains "lue"` finds *Blue Train*; `a matches "lue"` does not, as no word starts with it.
- Every stack writes the same SQL per database; the cases are in `conformance/queries.json`.

### Files

Code reaches files only in the file stores of the project settings, the folders that also keep pictures (`filestore photos "data/photos"`). What a program touches on disk is in its settings, and a store that differs per machine comes from an environment variable.

```clarity
// Photos dropped in the photos folder, named after the artist
// (Miles Davis.jpg), become the photos of artists that have none.
action findArtistPhotos()
    effects db.write, files.read
    let found = files f in photos where f.extension == "jpg" or f.extension == "png"
    let artists = find a in Artist where a.photo == none
    for artist in artists
        for f in found
            if f.name == artist.name + "." + f.extension
                artist.photo = f
                save artist

action exportCollection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- `files f in photos` lists every file of the store and its folders, leaving out files and folders whose name starts with a dot, sorted by path. `where` and `order by` work as in `find`, on the details of a file, and run in memory: the condition may be any bool.
- A file has `name` (`Blue Train.jpg`), `path` inside the store (`2024/Blue Train.jpg`, with `/` between folders), `extension` in lower case without the dot (`jpg`), `size` in bytes and `modified` (a date and time, in UTC). A list of files is `list<file>`, one file `file`.
- `read text of f` gives the text a file holds, as UTF-8. `write t to exports "lists/albums.txt"` writes one, making its folders. A path that would leave the store is refused.
- `read bytes of f` gives the contents of a file as they are, of type `bytes`: a picture, a PDF, anything that is no text. `write b to exports "copies/cover.png"` writes bytes as they are. `b.size` is how many bytes there are. Bytes are no window data: the screen has nothing to show of them, so code keeps them while it works and hands them to engines and functions.
- `zip(paths, texts)` gives the bytes of a zip archive with one file per path, holding the text at the same place as UTF-8, in that order and stored as it is: `write zip(["mimetype", "OEBPS/ch1.xhtml"], ["application/epub+zip", chapter]) to exports "book.epub"` writes an EPUB, whose `mimetype` must come first and uncompressed. The same files give the same bytes on every stack (conformance/zip.json).
- The picture functions give a new picture, as PNG, and leave their arguments as they were: `resize(p, width, height)`, `crop(p, x, y, width, height)` (clamped to the picture), `rotate(p, degrees)` (a quarter turn exactly; other angles on a transparent ground the size of the turned picture), `flip(p, horizontal)` (left to right with `true`, top to bottom with `false`), `blend(base, over, x, y, opacity)` (`over` laid on `base` at that place, its alpha times `opacity` from 0 to 100), `adjust(p, hue, saturation, brightness, contrast)` (hue turned by degrees, the others from -100 to 100, 0 leaving it as it is), and `pictureWidth(p)` and `pictureHeight(p)` in pixels. A picture is a record's picture (read from the database or its file store first) or bytes read from a file: `book.cover = resize(read bytes of f, 600, 900)`. No picture, bytes that are no PNG, JPEG or WebP picture, or a side of 0 or above 30000 pixels is refused with a message. Every stack gives the same pixels, through Clarity.Image (conformance/picture-functions.json); a program carries it only when its code calls one. On Go, C and C++ they run in a desktop program, which can also serve a browser; an engine has none.
- A file becomes a record's picture with `photo.picture = f`. In the field's own store (`store photos`) it is the file itself: the column holds its path, nothing is copied, as a catalog of photos wants. Kept anywhere else, the picture is a copy of the file. A file that is no picture is refused.
- Listing, reading and writing are effects, `files.read` and `files.write`, which an action declares; a function cannot reach files.

### Data formats

Code reads records from text in a format, and writes values in one, through one catalog of formats (docs/FORMATS-PLAN.md). Every stack calls the same implementation, Clarity.Formats, so a format reads and writes alike everywhere, to the byte.

```clarity
let orders = read list<Order> from json reply.text at "data.orders"
let order = read Order from json text
let body = json of order
```

- `read T from <format> text` reads one record (`T` an entity) or a list of them (`list<Order>`); `<format> of value` writes a record, a list of records or a value. The format is a name of the catalog; options follow the text, each as its name and value.
- The catalog:

| Format | Reads | Writes | Options |
| --- | --- | --- | --- |
| `json` | records, records inside records (`owner: User`, `lines: list<Line>` of a web API) | records, lists, values | `at "data.orders"` (reading): starts inside, at those object names |
| `csv` | records of one flat table | records, lists of records | `separator ";"` (any one character, `"\t"` for a tab; default `,`), `quote "'"` (default `"`), `noHeader true` (no header row: the columns are the entity's fields in order), `encoding "windows-1252"` (or `"iso-8859-1"`; default UTF-8) |
| `yaml` | records, records inside records, lists, values | records, lists, values | `at "data/orders"` (reading, and writing into the mappings it names) |
| `xml` | records, records inside records, lists | records, lists, values | `at "shop/orders/order"` (reading and writing, always when writing): the elements around the records and the record's own, `namespace "urn:example:orders"` (writing), `indent true` (writing), `encoding "windows-1252"` (writing; default UTF-8) |

- **YAML** is read as YAML 1.2 with its core schema: block and flow mappings and sequences, quoted and plain scalars, block scalars (`|`, `>`), comments, anchors and aliases, the first document of a stream. A plain scalar is a number, `true` or `false`, `null` or text, as the core schema says, and is then fitted as JSON's values are. Writing gives block YAML with two spaces of indent, quoting a text only where it would read back as something else. A YAML file source keeps the file as written: comments, order and quoting outside the records, and inside them every line no change touched.

- **CSV** follows RFC 4180: a field holding the separator, the quote or a line break is quoted, a quote inside doubled; lines end in CRLF or LF; a UTF-8 byte order mark is dropped. The header row names the columns, matched as names are; an empty field keeps the default; `true` and `false` fill a bool; a list of values is written as its JSON text (`["a","b"]`). One record is the one row there is. Writing gives a header row of the entity's fields that are one value (not the id, references, pictures, files or records inside), then a row per record, quoted where needed, lines ended by CRLF. CSV holds no records inside records, and `csv of` takes a record or a list of them, not a value.

- **XML** is read from where `at` says: element names with slashes between, as XPath writes them, the last one the record's element (`at "shop/orders/order"` takes every `<order>` inside `<shop><orders>`). Without `at`, the root element's children are the records. A field takes its value from a child element of its name, else from an attribute of its name, names matched as every format matches them; that order is fixed. An element's text is its value. The entity says what each element holds: a field that is a record takes the element of its name (`customer: Customer` from `<customer>`), and a list takes the elements of its name, repeated (`<tags>a</tags><tags>b</tags>`), or one element of its name whose children all share one name (`<lines><line/><line/></lines>`). Names match by their local name, so `<o:order>` is an `order` whatever its prefix. The encoding in the XML declaration is honoured (UTF-8, UTF-16 and the encodings CSV reads).
- Writing XML always takes `at`, for no element name is guessed: `xml of orders at "shop/orders/order"` writes `<shop><orders><order>...`, and `xml of order at "order"` one record as `<order>`. Without it the checker asks for it. Each field is written as a child element of its name; a list of values as repeated elements of the field's name; a list of records inside an element of the field's name, one element per record named after its entity in camelCase (`<lines><line>...</line></lines>`). A null value is left out, and the document starts with the XML declaration, so what Clarity writes it reads back the same. `indent true` writes one element per line, indented, for files people read; `namespace "urn:example:orders"` writes that namespace as the document's default, without a prefix; `encoding` writes in another encoding than UTF-8.

- **What is read is fitted to the entity.** A name matches by itself, else ignoring case, underscores, hyphens and spaces (`creditLimit` matches `credit_limit`, `CreditLimit` and `Credit Limit`), the first written counting when several do. Numbers and strings of numbers stand for each other (a string may have spaces around it), and so do `true` and `false` and their text; a number read as text keeps how it was written; a decimal keeps every digit and an exponent is written out (`1e3` is `1000`); a whole number is any 64-bit integer, also written `3.0` or `2e3`; a date and time keeps its date where a date is wanted; an enum value matches in any case. What is left out or null keeps its default; other names, `id` and the fields that are references, pictures, files and lists of stored records are not read. A record held inside, and a list of them, are read from the nested object and list, each fitted to its own entity. The records are new, so `save` adds them.
- **What does not fit stops the code** with a message that says where, the same words on every stack: `the text is no JSON`, `the JSON has no 'data.orders'`, `the JSON is not a list of Order`, `field 'lines' of Order, item 2: field 'quantity' of Line is an integer, not "three"`.
- **What is written is Clarity's one form**, the same bytes on every stack: a record's `id` first, then its fields in the entity's order; names as the fields are named; decimals and money as exact text in their shortest form (`12.50` is written `12.5`); whole numbers without a fraction; enums by name; dates as `2026-10-25` and dates and times as `2026-10-25T09:30:00`; no spaces.
- An option is given once, of the kind the catalog says (`the option 'at' of json is text, not integer`), an option writing always takes is given (`'xml of' needs 'at'`), and a format holds records inside records only where the catalog says it nests. Reading and writing have no effect, so a function may do it; an engine does not.
- A program carries Clarity.Formats only when its code reads or writes a format: Rust through the crate `clarity-formats`, C# through the NuGet `Clarity.Formats`, Go, C and C++ through its library beside Clarity.UI's (a Go program then builds with cgo).

### Mappings

A mapping says, for one case, how a kind of file is laid over an entity: where the records are, where each field is, the records inside, a schema to check against, and the format's options. It is the same declaration for every format.

```clarity
mapping OrderFeed of Order from xml
    at "shop/orders/order"
    number at "@number"
    customer at "customer/name"
    lines of Line at "lines/line"
        order is ".."
    check "orders.xsd"
    indent true
```

- `mapping Name of Entity from <format>`, then lines, each optional: `at "path"` (where the records are; else the format's own default), `field at "path"` (else the field is found by its name), `field of Entity at "path"` with the lines of the records inside, among them `field is ".."` (the field of theirs that refers to the record they sit in), `check "file"` (a schema file of the project, of a kind the format takes: `xsd` for XML) and the format's options from the catalog (`indent true`, `separator ";"`).
- Paths are written alike for every format: steps with slashes between, `@name` as the last step for an attribute where the format has them (XML), `.` for a record's own text, `..` for the record around. CSV has columns only: a path is a column's name, and a mapping from csv has no `at` and no records inside.
- Code reads and writes through it: `read list<Order> from text using OrderFeed`, `read Order from text using OrderFeed`, `text of orders using OrderFeed`. The format comes from the mapping, and the entity read or written is the mapping's. What writing always takes (XML's `at`) the mapping gives.
- A file data source names one in the project file: `datasource feed file "imports/orders.xml" external using OrderFeed`; the entity lives in that source (`source feed` in its module).
- `read list<Order> from xml text at "..."` and `xml of orders at "..."` are mappings written inline, and keep working.
- The checker holds a mapping to its entity and its format: an unknown entity or field, a field mapped twice, records inside that are not a list or record of the entity named, `is ".."` on a field that is not a `ref` to the record around, a path the format cannot have, an option the format lacks or of the wrong kind, `check` on a format without schemas or a schema file missing from the project's folder, and a mapping used with another entity.

### Calling web APIs

Code calls a web API with `http`, and gets its reply.

```clarity
action sendOrder(order: Order, token: text)
    effects net
    let reply = http post "https://api.example.com/orders" with order.summary header "Authorization": "Bearer {token}"
    if not reply.ok
        refuse @orderNotSent(status = reply.status)

code Status
    handler refreshClicked
        let reply = http get "https://status.example.com/api/now"
        statusText = reply.text
```

- `http get url`, `http delete url`, `http post url with body`, `http put url with body` and `http patch url with body`. The url and the body are text; only post, put and patch send a body. `header "Name": value` adds a header, as many as needed.
- The reply has `status` (200, 404, and 0 when no answer came), `ok` (true for a status from 200 to 299) and `text` (what the service answered, or why no answer came). Its type is `http reply`; its details are read, not set.
- A call does not stop the code: a server that is not there, a wrong name or a timeout of 30 seconds is a reply with status 0, so the code decides what a failure means. Only `http` and `https` addresses are called; redirects are followed.
- Calling out is the effect `net`, which an action declares. Handlers and commands call out as they are; a function, an engine and a data event cannot.
- `http get "/orders" from shop` calls a path of the web API data source `shop`: at its address, signed with its `auth` (`bearer` sends `Authorization: Bearer <secret>`, `api-key` `X-Api-Key: <secret>`, `basic` `Authorization: Basic ...`). `query "status": status` adds a query parameter, encoded, as many as needed.
- `when` after a header or query pair leaves it out of the call when its condition does not hold: `query "limit": "{limit}" when limit != 0`. With a default, it sends an optional value only when one is given.
- `http get "/orders" from shop every page` reads a long list a page at a time, as the source's `paging` line says, and gives one reply: its text is one JSON list of the items of every page. Only `get` reads every page, and only from a source that pages. A page that is not ok is the reply; after 1000 pages the reply has status 0.
- **Records inside records.** An entity of a web API holds the records its service nests: `owner: User` holds one, `lines: list<Line>` several, and code reads them as any record and list (`order.owner.name`, `for line in order.lines`); `json of` writes them nested too. What it holds lives in the web API as well, and a record never holds itself, through any number of records: a list may, as it can be empty. A stored entity refers to another with `ref User` instead, and keeps its lists in their own table.
- A key does not belong in the code: a header named `Authorization`, or one that holds `key`, `token` or `secret`, with a plain text as its value is a warning. Keep the key in a setting and put it in with a slot.
- `json of order` writes a record as JSON, and `read list<Order> from json reply.text at "data"` reads a service's JSON into records: see [Data formats](#data-formats). A post sends a record with `http post url with json of order header "Content-Type": "application/json"`.

### Types

The types of values follow the data dictionary:

| Type | Values of fields of type |
|---|---|
| `text` | text, email, url |
| `integer` | integer, id |
| `decimal` | decimal, money |
| `bool` | bool |
| `date`, `datetime` | date, datetime |
| `duration` | duration; durations add up (`+`, `-`), scale by a number (`*`, `/`), compare, and take a number of seconds: `let total: duration = 0` |
| an enum | that enum |
| a record | an entity, a `ref` |
| a list of records | `list<Entity>`, a list in the window's data or one a query finds; code builds one too, stored records or records made in code alike: `let found: list<Project> = []`, then `found = found + [p]` |
| a list of values | `list<text>`, `list<integer>`, `list<decimal>`, `list<bool>`, `list<date>`, `list<Enum>`: in code, in what an engine keeps and in list fields; a list is a value, so a copy changes on its own |
| `image` | an `image` field; compared with and set to `none`, or set to a file |
| `file`, `list<file>` | a file of a file store, and a list of them; see [Files](#files) |
| `bytes` | the contents of a file as they are (`read bytes of f`); `size` is their number |

An integer is accepted where a decimal is expected. Nothing else converts by itself. The checker reports unknown names, wrong types, setting something that cannot be set, and calls with the wrong arguments.

## 11. A complete example

A project file, one module and the files of its app and windows: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer.

`shop.clarity`:

```clarity
project Shop
    stack rust desktop
    datasource main sqlite "data/shop.db"
```

`shop/customers.clarity`:

```clarity
module customers

enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness

entity Customer
    name: text(80), required, label @customerNameLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vatNumber: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    notes: text, label @customerNotesLabel

code Customer
    before save
        // Only business customers keep a VAT number, whichever window saves them.
        if customer.kind == Private
            customer.vatNumber = ""

code CustomerBrowse
    handler windowReturned
        // The form may have saved a customer.
        filterChanged()

code CustomerBrowseView
    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text order by c.name

    handler openClicked
        open CustomerForm with customer = grid.selected

code CustomerFormView
    handler kindChanged
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        vatNumber.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        if customer.kind == Business and customer.vatNumber == ""
            vatNumber.error = @neededForBusinessCustomers
            return
        save customer
        message @savedName(name = customer.name)
        close

    handler closeClicked
        close
```

Beside it in `shop/design/` are the app and the windows, each a `.cxaml` file of its own. Each window shows its parts in a view. `open CustomerForm with customer = ...` fills the data of the view the window shows.

`Shop.cxaml`, the app, which starts at the customer list:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<App class="customers.Shop" main="CustomerBrowse">
</App>
```

`CustomerBrowse.cxaml` and `CustomerBrowseView.cxaml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Window class="customers.CustomerBrowse" title="@customerBrowseTitle" onreturn="windowReturned">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="CustomerBrowseView" />
  </Content>
</Window>
```

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="customers.CustomerBrowseView">
  <Data name="customers" type="list&lt;Customer&gt;" />
  <Label name="nameFilterLabel" at="1,1" span="2" caption="@nameFilterLabel" for="nameFilter" />
  <Textbox name="nameFilter" at="3,1" span="4" placeholder="@nameFilterPlaceholder" onchange="filterChanged" />
  <Table name="grid" at="1,2" span="12" rows="8" bind="customers" columns="name, kind" onopen="openClicked" />
  <Button name="newCustomer" at="9,10" span="2" caption="@newCaption" opens="CustomerForm" />
  <Button name="openButton" at="11,10" span="2" caption="@openButtonCaption" onclick="openClicked" />
</View>
```

`CustomerForm.cxaml` and `CustomerFormView.cxaml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Window class="customers.CustomerForm" title="@customerFormTitle">
  <Content name="main" at="1,1" span="12" rows="6">
    <View name="CustomerFormView" />
  </Content>
</Window>
```

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="customers.CustomerFormView">
  <Data name="customer" type="Customer" />
  <Form name="customerForm" at="1,1" span="12" rows="4" bind="customer">
    <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
    <Textbox name="name" at="4,1" span="6" bind="customer.name" />
    <Help name="nameHelp" at="10,1" span="3" for="name" />
    <Label name="kindLabel" at="1,2" span="3" caption="@kindLabel" for="kind" />
    <Radio name="kind" at="4,2" span="6" bind="customer.kind" none="orientation" onchange="kindChanged" />
    <Label name="vatNumberLabel" at="1,3" span="3" caption="@vatNumberLabel" for="vatNumber" />
    <Textbox name="vatNumber" at="4,3" span="4" bind="customer.vatNumber" />
    <Help name="vatNumberHelp" at="8,3" span="5" for="vatNumber" />
    <Label name="notesLabel" at="1,4" span="3" caption="@notesLabel" for="notes" />
    <Textbox name="notes" at="4,4" span="9" bind="customer.notes" />
  </Form>
  <Button name="saveButton" at="9,6" span="2" caption="@saveButtonCaption" key="Ctrl+S" default="true" onclick="saveClicked" />
  <Button name="closeForm" at="11,6" span="2" caption="@closeFormCaption" cancel="true" onclick="closeClicked" />
</View>
```

The full shop example, with orders, a menu and a ribbon, is `examples/shop.clarity`.

## 12. Grammar summary

A compact view of the grammar. `[x]` is optional, `x*` is zero or more, `a | b` is one of them, and an indented line is a line in the block above it. Words in quotes are written as shown.

```text
project-file  = "project" Name
                    setting*
setting       = "title" String | "stack" word word word+ | "output" String | "language" String | "currency" word | "theme" Name+
              | "datasource" name kind String ["external"] ["readonly"]
                    ["auth" kind String]

module-file   = "module" name
                ("use" name)*
                ["source" name]
                declaration*
declaration   = enum | entity | settings | theme | code | function | action | engine | cli | command | custom-part | library

enum          = "enum" Name
                    (Value ["label" String])+
entity        = "entity" Name
                    ["table" String] ["key" String]
                    (name ":" type ("," modifier)*)+
type          = "text" ["(" n ")"] | "decimal" "(" p "," s ")" | "integer" | "money" | "bool"
              | "date" | "datetime" | "email" | "url" | "id" | Enum | "ref" Entity | "list" "<" Entity ">"
modifier      = "required" | "unique" | "owned" | "cascade" | "computed"
              | ("min" | "max" | "default") value | ("pattern" | "label" | "help" | "column") String | "format" name

page-file     = <Root class="module.Name" root-attr*>            // design/Name.cxaml, one per page
                    <Data name="name" type="type-ref" [default="value"] />*
                    <Tells message="name[(params)]" />*                   // a view
                    region-element* part-element*
                </Root>
Root          = "Window" | "Dialog" | "View" | "Splash" | "Toolbox"
root-attr     = title="String" | width="n" height="n" | columns="n" | cell="n" | style="name"
              | gives="type"                                             // a dialog
              | ("onopen" | "onclose" | "onreturn")="handler"
part-element  = <Kind attribute*> (part-element | content-view | page-element)* </Kind>   // or <Kind ... />
content-view  = <View name="Name" ["on" tell="handler"]* />            // inside a Content
page-element  = <("Tab" | "Step" | "Section") title="String"> part-element* </...>   // a tab, step or section
attribute     = property="value"                                        // see the table in section 6
custom-part   = "part" Name
                    ("native" String | "nuget" String version | "dll" String)
                    ["binds" type-ref] ["size" n "x" n] ["grows"] ["preview" String]
                    (name ":" type-ref ("," ("default" | "min" | "max") value)*)*
                    (("tells" | "hears") name ["(" params ")"])*
library       = "library" Name
                    ("nuget" String version | "dll" String | "crate" String String | "go" String version | "cmake" String)
                    ("function" name "(" params ")" ["->" type-ref])*
app-file      = <App class="module.Name" app-attr*>                     // design/Name.cxaml, one per project
                    <Data name="name" type="type-ref" [default="value"] />*
                    region-element*
                </App>
app-attr      = title="String" | main="Window" | splash="Page" | splashFor="seconds"
              | ("onstart" | "onunconnected")="handler"
region-element = <Menu name="name"> frame-command* </Menu> | <Toolbar name="name"> frame-command* </Toolbar>
              | <Ribbon name="name" [collapsible="true"] [collapsed="true"]> (<Tab title="String" name="name"> (<Group title="String" name="name"> frame-command* </Group>)* </Tab>)* </Ribbon>
              | <Sidebar side=("left" | "right") name="name" [width="n"] [min="n"] [max="n"] [resizable="true"] [collapsible="true"] [collapsed="true"] [maximizable="true"] [columns="n"]> part-element* </Sidebar>
              | <Footer [status="String"] columns="n"> part-element* </Footer>
frame-command = <Item [kind=("button" | "separator")] [caption="String"] [name="name"] [icon="name"] [key="String"] [large="true"]
                    [opens="Target" [newWindow="true"] | onclick="handler"]> frame-command* </Item>

code          = "code" (Window | App)
                    ("handler" name ["(" [param ("," param)*] ")"]
                        statement*)*
              | "code" Entity
                    (("before" | "after") ("save" | "delete")
                        statement*)*
function      = ("function" | "action") name "(" [param ("," param)*] ")" ["->" type-ref]
                    ("require" expr | "ensure" expr | "effects" effect ("," effect)*)*
                    statement*
param         = name ":" type-ref ["=" expr]                  // a default: the parameters with one come last
call          = name "(" [expr ("," expr)*] ["," named ("," named)*] ")"   // or only named values
named         = name ":" expr
http          = "http" ("get" | "post" | "put" | "delete") expr ["from" name] ["with" expr]
                    (("header" | "query") String ":" expr ["when" expr])*

engine        = "engine" Name "from" String
                    ["uses" service ("," service)*]
                    ["says" Key ("," Key)*]
                    ["limit" n ("seconds" | "MB") ["," n ("seconds" | "MB")]]
                    ["state" "per" ("app" | "window")]
                    ["on" "open"]
                    ("function" name "(" [param ("," param)*] ")" "->" type-ref)*
              | "engine" Name                                // written in Clarity
                    ["uses" "random"]
                    ["state" "per" ("app" | "window")]
                    ("keep" name ":" type-ref "=" expr)*
                    function*
service       = "clock" | "random" | "log" | "language"
cli           = "cli" Name
                    ["help" text]
command       = "command" [group] name ["(" [param ("," param)*] ")"]
                    ("help" text | "option" name ":" type-ref
                        ("," ("default" expr | "min" expr | "max" expr | "required" | "help" text))*)*
                    statement*                                // also "print" expr ["columns" field ("," field)*] and "exit" expr
command-ask   = "ask" text ("yes" text "no" text | "for" type-ref ["default" expr] ["from" expr])
engine-call   = Engine "." name "(" [expr ("," expr)*] ")"
list-of       = "[" [expr ("," expr)*] "]"
item          = expr "[" expr "]"

find          = "find" name "in" Entity ["where" condition]
                ["order" "by" name "." field ["desc"] ("," name "." field ["desc"])*]

theme         = "theme" Name
                    ("title" String | "mode" ("light" | "dark" | "system") | "color" name String ["dark" String]
                    | "font" [role] String | "size" [role] n | "radius" [kind] n | "padding" n | "border" n
                    | "shadow" ("none" | "flat" | "raised")
                    | "part" kind [state] look | "style" name look)*
look          = ("background" | "text" | "border") (name | String) | "radius" n | "weight" ("bold" | "normal")
                    | "size" n | "font" role | "align" ("left" | "center" | "right")
                    ("," look)*
translation   = "translation" language
                    (String String)*
migration     = "migration" String ["baseline"]
                    (("add" "entity" Entity
                        field*)
                    | "remove" "entity" Entity | "rename" "entity" Entity "to" Name
                    | "add" "field" Entity "." field | "remove" "field" Entity "." name
                    | "rename" "field" Entity "." name "to" name | "change" "field" Entity "." name ":" type
                    | ("add" | "remove") "unique" Entity "." name
                    | "map" ("table" Entity | "key" Entity | "column" Entity "." name) "to" (String | "default"))*
```

## 13. Diagrams

Diagrams are Mermaid files in the project's `diagrams/` folder. An ERD and a class diagram map onto the project: they propose changes, and Apply makes them. The other kinds (state, sequence, activity, architecture) are drawn only. A project keeps one ERD, and data changes are written into it.

### ERD

```
erDiagram
    Customer {
        text(80) name "required"
        email email UK
        CustomerKind kind
        text[] tags
    }
    Order {
        text(12) number UK "required"
        Customer customer FK "required"
        money total "computed"
        decimal discount "decimal(5, 2), min 0"
    }
    OrderLine {
        Order order FK "required"
        integer quantity "default 1"
    }
    Order }o..|| Customer : customer
    OrderLine }o..|| Order : order
    %% clarity: Customer at 24,109
    %% clarity: Order at 408,93
```

- An entity block is an entity; an attribute `type name` is a field in Clarity's types (`text(80)`, `money`, `email`, an enum's name), a list as `text[] tags`. Clarity adds `id` itself; names are camelCase.
- `UK` is `unique`. The comment holds every other modifier as the module writes it (`"required"`, `"computed"`, `"min 0"`, `"default 1"`, `"owned"`), and a type Mermaid's word cannot hold, first (`"decimal(5, 2), min 0"`). An attribute left without its modifiers proposes removing them.
- A ref is an attribute with `FK` (`Customer customer FK "required"`) and a line to its entity named after the field: `Order }o..|| Customer : customer`, the many side first. Exactly one (`||`) makes it required, zero or one (`o|`) leaves it optional. An identifying line (`--`) also gives the one side an owned list, a non-identifying one (`..`) a plain list when the one side lists them.
- The `%% clarity:` lines say where each box stands (and how a line bends): keep them when writing the diagram again.

### Class diagram

```
classDiagram
    namespace customers {
        class Customer {
            +name: text(80), required
            +email: email, unique
            +kind: CustomerKind
        }
        class CustomerKind {
            <<enumeration>>
            Private
            Business
        }
    }
    namespace sales {
        class Order {
            +number: text(12)
            +customer: Customer
        }
    }
    class sales {
        <<module>>
        +totalOf(order: Order) money
    }
    Order "*" --> "1" Customer : customer
    sales ..> Customer
```

- Each class stands in the namespace of its module and is named by its own name everywhere, in relations too: `Order`, never `sales.Order`. A class name is written once.
- An attribute carries its type and modifiers as the module writes them (`+name: text(80), required`, `+total: money, computed`); one left without its modifiers proposes removing them.
- A class with attributes is an entity; `<<enumeration>>` with its values is an enum; `<<engine>>` with methods is an engine; a `<<module>>` class, named as its module and outside the namespaces, holds the module's functions and actions.
- Keep the `%% clarity:` lines, which say where each box stands, when writing the diagram again.
- `A "*" --> "1" B` is `b: ref B` on A; a composition `*--` is an owned list, an aggregation `o--` a plain list. A uses arrow from a `<<module>>` or `<<engine>>` (`sales ..> Customer`) is drawn only.
