# Languages and formats

How to translate an application: texts by key and their slots, the language files and their rules, Clarity's own words, keys that point at resources per language, letting people choose the language, and how numbers, dates and money show.

## The project's language

The project file says which language its texts are written in, and the currency of `money` fields:

```clarity
project Shop
    stack rust desktop
    language "en"
    currency EUR
```

The language is a tag such as `"en"` or `"nl-NL"`; without the line it is `en`. The currency is an ISO 4217 code; without it, amounts show without a currency sign. Both are also under **Language, formats and theme** on the Project tab. See [The project file](project.file.md).

## Texts and keys

Every text people read is a key, written with `@`: a title, a caption, a label, a help text, a placeholder, a tooltip, a tab's title, an enum value's label, a menu item, the status text, a message, a refusal, and a caption or an error that code sets.

In the window's file, `shop/design/CustomerCard.cxaml`, a key is an attribute value that starts with `@`:

```xml
<Window class="shop.CustomerCard" title="@customerCardTitle">
  <Button name="saveCustomer" at="9,12" span="2" caption="@saveCaption" onclick="saveClick" />
</Window>
```

And in the module's code, `shop/shop.clarity`:

```clarity
code CustomerCard
    handler saveClick
        save customer
        message @savedName(name = customer.name)
```

A file per language, in the `translations` folder of the module folder, gives every key its text in that language. The project's language (`language "en"`) is the default, and has a file too:

```clarity
// shop/translations/en.clarity
translation en
    customerCardTitle "Customer card"
    saveCaption "Save"
    savedName "Saved {name}"
    yes "Yes"
    no "No"
```

```clarity
// shop/translations/nl.clarity
translation nl
    customerCardTitle "Klantkaart"
    saveCaption "Opslaan"
    savedName "{name} opgeslagen"
    yes "Ja"
    no ""
```

You type a text in the designer, and Clarity makes its key from the part and what the text is to it: `saveCaption`, `customerNameLabel`, `customerCardTitle`. Rename it under the field in Properties. A new key goes into every language file at once; Clarity does that for you.

### `translation`

```text
translation language
    key "text"
    key resource "file"
```

- The first line is `translation` and the file's language; the file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`. A file whose first line says another language is an error ("this file is for language 'nl' (its name), but it says 'de'"), and so is a first line that is no `translation` line.
- Each line below is a key and its text in quotes, or a key and the resource it points at (see [Keys that point at resources](#keys-that-point-at-resources)). Any other line is an error: "write a key and its text in quotes, saveCaption \"Save\", or the resource it points at, gameRules resource \"rules.md\"".
- A key is camelCase, as names are ("key 'Save_caption' is not camelCase, as names are"), and once per file. It is used anywhere in the project.

The checker also says:

- The default language has a file: "the default language en has no file; add translations/en.clarity".
- Every language file has the same keys: a key another language lacks is an error ("key 'saveCaption' is missing in nl; every language file has the same keys"), and so is a key the default language lacks ("key 'x' is in nl but not in en, the default language").
- The default language gives every key it uses a text: "key 'saveCaption' has no value in en, the default language". Another language may leave a key empty (`no ""` above): it shows the default's text until it is translated.
- Each language keeps the slots of the default's text and may move them: a slot left out ("savedName in nl must keep the slot {name}") or added is an error.
- A key no design or code uses is a warning: "key 'oldCaption' is not used anywhere; it can go".
- A text with letters written in quotes where people read it is an error that asks for a key: "\"Save\" is read by people, so it is a key: write @key, with its text in the language files". A text without letters, such as a calculator's `"7"` or `"+"`, reads the same in every language and stays as it is.
- A field's label, an enum value's label and the title of a theme that is offered are keys too; without one, the checker says which key to give (see [Entities and enums](data.entities.md)).

## Texts by key and their slots

A key's text names its values in slots, and code gives each slot its value by name, so a translator can move them and each value shows in the person's format:

```clarity
code OrderForm
    handler approveClicked
        approve(order)
        message @orderApproved(number = order.number, total = order.total)
        approveButton.caption = @approvedOn(day = order.orderedOn)
```

```clarity
translation en
    orderApproved "Order {number} of {total} is approved"
    approvedOn "Approved on {day}"
```

- A key is `@` and its name; with slots, `@key(slot = value, ...)` gives each slot of its text a value. Every slot the default language's text has gets one ("@savedName needs a value for {name}"), and no other ("@savedName has no slot {total}").
- A value is checked like any expression. A whole record or list is no value to show: `customer = order.customer` is an error that suggests one of its fields ("a slot shows a value, and customer is Customer; name one of its fields").
- A value shows as its field shows it: money with its currency, a date in the locale and the field's `format`, an enum value by its label. A local or a parameter shows as its type does.
- The slot `{app.version}` is the project's version, which Clarity fills itself: a use gives it no value, and a caption shows it as it is (`welcomeVersion "Version {app.version}"`). It needs the project file's `version` line.
- A keyed text is a text like any other: code keeps it in a local, passes it to a function, returns it, and puts it in a text value.
- Where it goes straight to the screen (a message, a refusal, a caption, an error, a text value a handler sets), the program sends the key with each value, and the screen shows its text in the person's language and fills the slots in the person's format.
- Anywhere else, such as a function's result, the program takes the key's text in the language the screen picked and fills its slots plainly.
- Text joined with `+` in a message, a refusal, a caption or an error is a warning that suggests one text with slots, since such text cannot be translated and its values do not follow the locale.

```clarity
function creditNote(order: Order) -> text
    if withinCredit(order)
        return @withinCredit(number = order.number)
    return @overCredit(number = order.number)
```

```clarity
translation nl
    withinCredit "Bestelling {number} blijft binnen het kredietlimiet"
    overCredit "Bestelling {number} gaat boven het kredietlimiet van de klant"
```

An engine lists the keys of the texts it gives with `says`; see [Engines](code.engines.md).

### Text with slots in code

A string in code may have slots too: `"{album.title}.txt"`, with `{{` and `}}` for braces. It is filled plainly, in no language, for what people do not read: a file's name, a line written to a file, a value to compare. Where people read it, it is a key.

## Clarity's own words

Clarity's parts and windows say some things themselves, by keys of their own. A project that uses what says them gives them their texts in its language files, as any key. Until the default language has one, a warning says so and the program shows Clarity's English: "Clarity's word 'yes' is not in en yet; it shows \"Yes\" until it is".

- `yes` and `no` are how the screen shows a bool; `ok` and `cancel` the buttons of Clarity's dialogs, and `dismiss` closes a message.
- `noPicture`, `loadPicture` and `removePicture` are what an image part bound to a picture shows of its own; `noFile`, `chooseFile`, `openFile`, `saveFileCopy`, `removeFile`, `fileCopySaved` and `fileCopySavedAt` a file part's.
- `languagePicker` and `themePicker` head the pickers that the click targets `language` and `theme` open (`languagePicker` also heads the sign-in part's language dropdown).
- `showSecret`, `hideSecret` and `copySecret` name the eye and the copy button of a password input.
- `chooseFolder` is the folder part's button.
- `addTag` and `removeTag` are the tags part's; `markdownWrite` and `markdownPreview` a Markdown memo's.
- `fieldRequired` ("{field} is required") refuses a save while a required field is empty, and `savedSince` a save of a record someone else changed since.
- `keysTitle`, `keysEverywhere`, `keysChange`, `keysReset`, `keysResetAll`, `keysClose`, `keysPress`, `keysNone`, `keysTaken`, `keysSwap`, `keysAlso`, `keysTyping`, `keysTab`, `keysEnter` and `keysEscape` are the words of the keys window, in a project that opens it (`opens="keys"`).
- `unconnectedNotKept` and `unconnectedFailed` say why the databases did not open: the connection is not set yet, or it did not work, with the database's own message in `{problem}`. A project whose database connects with a secret setting, or that handles `on unconnected`, has them.
- `signinRefused`, `signinTooManyTries`, `notAllowed`, `notYours`, `secretTooShort`, `signinRemember` and the sign-in parts' words come with signing in; see [Signing in](project.signin.md).
- `agentReadsWhileClosedSetting` and `agentChangesWhileClosedSetting` label Clarity's settings for agents; see [Settings](data.settings.md).

## Making a project's texts keys

A project written before texts had keys gets them with **Make texts keys**, under Problems, or with `clarity spec keys <project folder>`. Each text becomes a key named after where it is (`saveCaption`, `customerNameLabel`, `cardTitle`), its text the default language's value; a translation file that translated texts by what they said gives the same keys their translations.

To see what each language still leaves empty without opening Clarity, run `clarity spec texts <project folder>`: it lists, per language, each key without a text as a line to fill in.

## i18n in the project tree

The project tree lists the languages under i18n by name: first the default, then each other language. Click one to open it.

- The default language lists every key with where it is used and its text, which you can change; each needs one. A mark per other language says whether it translates the key, and a button per language opens it.
- Another language lists every key with where it is used, the default's text, and its own to type; it says how many keys it translates. A key left empty shows the default's text. A key still the same as the default's text counts as not translated, as the agents' missing_translations lists it; keep it where the language says it the same way (such as "OK").
- **Find a key or a text** narrows the list to what holds what you type; **Missing only** to the keys still to translate, empty or the same as the default.
- **+ New language** asks for the language, such as `nl` or `fr-BE`, in a dialog; Clarity writes its file, `translations/nl.clarity`, with every key, and opens it.
- **Language** on the Design and Preview ribbons shows the canvas and Preview in one of the project's languages, or the system's.

## Keys that point at resources

A key's value can be a file of the resources instead of a text, a file per language. A markdown part (`markdown @gameRules`) or an image (`image @bannerPicture`) shows by its key the file each language points it at:

```clarity
// noughts/translations/nl.clarity
translation nl
    gameRules resource "rules-nl.md"
    bannerPicture resource "banner-nl.png"
```

- Such a key points at a file in every language: a markdown's at a Markdown file, an image's at a picture. A key that is a text in one language is an error there ("@gameRules is a text in nl; a markdown shows a Markdown file: point the key at one, as gameRules resource \"rules.md\""), and so is a file of the wrong kind or a file that is not in the resources folder ("@gameRules points at rules-de.md in de, which is not in the resources folder").
- A part that shows text cannot use a key that points at a file ("@gameRules points at rules-nl.md in nl; a label shows text").
- Long texts belong in the resources, as Markdown, not in the language files.
- A new language starts each such key at the default language's file.
- In a language's list, such a key has a choice of the resources of its kind (Markdown files for a markdown, pictures for an image) in place of a text.

A part that shows the same file in every language names it by its `source` instead: `markdown source "license.md"`, `image source "logo.png"`. In the properties these are **Source** for one file in every language and **Key** for a file per language. The resources themselves are in [Pictures and files](data.pictures-files.md).

## Which language shows

The application shows the language for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else another region of Dutch, else the default language.

To let people choose, give a menu item or a button the click target `language`:

```xml
<Item caption="@languageItem" name="languageItem" opens="language" />
```

It lists the project's languages, each by its own name (Nederlands, Deutsch). The application switches at once and remembers the choice for its next start. The `language` setting holds the same choice; see [Settings](data.settings.md).

## Formats

Dates, times, numbers and money show the way the person using the application writes them. The locale comes from their system, or from the project's `language` where the system names none. The value stored is the same in every locale.

A field can pick a format by name with `format`; the entity editor has a Format column for it.

| Type | Default | Formats |
|---|---|---|
| `integer` | with thousands separators | `plain`: without them, for codes and years |
| `decimal(p, s)` | with `s` decimals | `plain`; `percent`: `0.25` shows `25%` |
| `money` | with the currency and its decimals | `plain`: without the currency |
| `date`, `datetime` | `short`, in digits | `long`: `17 October 2026`; `iso`: `2026-10-17` everywhere |

```clarity
entity Invoice
    issued: date, format long, label @invoiceIssuedLabel
    rate: decimal(5, 4), format percent, label @invoiceRateLabel
    year: integer, format plain, label @invoiceYearLabel
```

A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5. Values in slots follow the locale too: `message @savedOn(day = order.placed)`.

## On each stack

Every stack shows the same texts and formats. The program carries the translations, and Clarity.UI shows each text in the person's language; C# also gets a `.resx` file per language and a typed `Texts` class with a property per key. The cases in `conformance/formats.json` pin what every stack shows and reads.

See also [Entities and enums](data.entities.md), [Settings](data.settings.md) and [Pictures and files](data.pictures-files.md).
