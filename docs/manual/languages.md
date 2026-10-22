# Languages and formats

How to translate an application, let people choose its language, and control how numbers, dates and money show.

## The project's language

The project file says which language its texts are written in, and the currency of `money` fields:

```clarity
project Shop
    stack rust web desktop
    language "en"
    currency EUR
```

The language is a tag such as `"en"` or `"nl-NL"`. The currency is an ISO 4217 code; without it, amounts show without a currency sign. Both are also under **Language, formats and theme** on the Project tab.

## Texts and keys

Every text people read is a key, written with `@`: a title, a caption, a label, a help text, a placeholder, a tooltip, a tab's title, an enum value's label, a menu item, the status text, a message, a refusal, and a caption or an error that code sets.

```clarity
page CustomerCard: window
    title @customerCardTitle
    button @saveCaption name save at 9,12 span 2 on click: saveClick

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

- The file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`.
- Every language file has the same keys. A key one file lacks is an error, so a new key goes into every file at once; Clarity does that for you.
- The default language gives every key a text. Another language may leave a key empty, as `no ""` above: it shows the default's text until it is translated.
- A key is camelCase, as names are. You type a text in the designer, and Clarity makes its key from the part and what the text is to it: `saveCaption`, `customerNameLabel`, `customerCardTitle`. Rename it under the field in Properties.
- A text people read that is written in quotes is an error that asks for a key. A text without letters, such as a calculator's `"7"` or `"+"`, reads the same in every language and stays as it is.
- `yes` and `no` are how the screen shows a bool; an image part bound to a picture has `noPicture`, `loadPicture` and `removePicture`. Every project gives them their texts.

## Slots

A key's text names its values in slots, and code gives each slot its value by name:

```clarity
function credit_note(order: Order) -> text
    if within_credit(order)
        return @withinCredit(number = order.number)
    return @overCredit(number = order.number)
```

```clarity
translation nl
    withinCredit "Bestelling {number} blijft binnen het kredietlimiet"
    overCredit "Bestelling {number} gaat boven het kredietlimiet van de klant"
```

Each language keeps the slots of the default's text and may move them. A text that goes to the screen gets its values in the person's format; a text a function returns gets them plainly. An engine lists the keys of the texts it gives with `says`; see [Engines](code.engines.md#texts-and-languages).

## Making a project's texts keys

A project written before texts had keys gets them with **Make texts keys**, under Problems, or with `clarity spec keys <project folder>`. Each text becomes a key named after where it is, its text the default language's value; a translation file that translated texts by what they said gives the same keys their translations.

To see what each language still leaves empty without opening Clarity, run `clarity spec texts <project folder>`: it lists, per language, each key without a text as a line to fill in.

## i18n in the project tree

The project tree lists the languages under i18n by name: first the default, then each other language. Click one to open it.

- The default language lists every key with where it is used and its text, which you can change; each needs one. A mark per other language says whether it translates the key, and a button per language opens it.
- Another language lists every key with where it is used, the default's text, and its own to type; it says how many keys it translates. A key left empty shows the default's text.
- **Find a key or a text** narrows the list to what holds what you type; **Missing only** to the keys still to translate.
- **+ New language** asks for the language, such as `nl` or `fr-BE`, in a dialog; Clarity writes its file, `translations/nl.clarity`, with every key, and opens it.
- **Language** on the Design and Preview ribbons shows the canvas and Preview in one of the project's languages, or the system's.

## Keys that point at resources

A markdown part or an image can show a file of the resources by a key, a file per language: `markdown @gameRules`. In each language the key's value is the file, written `gameRules resource "rules-nl.md"` in the language file. In a language's list, such a key has a choice of the resources of its kind (Markdown files for a markdown, pictures for an image) in place of a text. A new language starts it at the default language's file.

Resources sits under i18n in the project tree: every file of the `resources` folder. Click one to open it: a Markdown file to write, with how it reads beside it (**Save**, or Ctrl+S), or a picture. Each says which keys point at it. **+ New Markdown file** asks for a name and writes `resources/<name>.md`; **+ Add picture** copies a picture you pick into the folder. A markdown or image part names its file in the properties: **Source** for one file in every language, **Key** for a file per language.

## Which language shows

The application shows the language for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else another region of Dutch, else the default language.

To let people choose, give a menu item or a button the click target `language`:

```clarity
item @languageItem name languageItem on click -> language
```

It lists the project's languages, each by its own name (Nederlands, Deutsch). The application switches at once and remembers the choice for its next start.

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
    issued: date, format long
    rate: decimal(5, 4), format percent
    year: integer, format plain
```

A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5. Values in slots follow the locale too: `message @savedOn(day = order.placed)`.

## On each stack

Every stack shows the same texts and formats. On Rust, Go, C and C++ the web renderer carries the translations and fills in the formats; C# with Avalonia gets a `.resx` file per language and formats with .NET's culture data, pinned to the same answers.

See [Entities](data.entities.md) for fields, and [the language reference](../LANGUAGE.md#translations) for the full rules, with formats under [Formats](../LANGUAGE.md#formats).
