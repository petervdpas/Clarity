# Languages and formats

How to translate an application, let people choose its language, and control how numbers, dates and money show.

## The project's language

The project file says which language its texts are written in, and the currency of `money` fields:

```
project Shop
    stack rust web desktop
    language "en"
    currency EUR
```

The language is a tag such as `"en"` or `"nl-NL"`. The currency is an ISO 4217 code; without it, amounts show without a currency sign. Both are also under **Language, formats and theme** on the Project tab.

## Translations

Every text a person sees is in the project already: labels (also those derived from names), help texts, titles, captions, placeholders, tooltips, tab titles, enum labels, menu items, the status text, messages, refusals, and captions and errors set in code. Clarity collects them, so you never mark a text for translation.

A file per language, in the `translations` folder of the module folder, gives them in that language:

```
// shop/translations/nl.clarity
translation nl
    "Customer card" "Klantkaart"
    "Saved {customer.name}" "{customer.name} opgeslagen"
    "Yes" "Ja"
```

- The file's name is its language: `nl.clarity` says `translation nl`, `nl-BE.clarity` says `translation nl-BE`.
- Each line is the text as written, then its translation. A text is found by what it says, for the whole project.
- A translation keeps the slots of its text and may move them. A slot missing or added is an error.
- A text without a translation shows as written, so a language can grow text by text.
- When a text changes, its old translation no longer applies. The checker warns that the old line is not shown anywhere, and the new text shows as written until it has a translation.

## The Translations panel

The Translations panel is at the bottom of the Project tab. It lists every text with where it is used and a column per language, and says per language how many texts miss a translation. What you type goes into the translation files.

- **Add language** starts a new language from a tag such as `nl`.
- **Only texts that miss a translation** narrows the list.
- **Show the designer in** shows the canvas and Preview in one of the project's languages, or in the system's.

## Which language shows

The application shows the translation for the locale of the person using it: `nl-BE` takes `nl-BE.clarity`, else `nl.clarity`, else another region of Dutch. The project's own language shows as written.

To let people choose, give a menu item or a button the click target `language`:

```
item "Language" on click -> language
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

```
entity Invoice
    issued: date, format long
    rate: decimal(5, 4), format percent
    year: integer, format plain
```

A number input reads what is typed in the same locale: in Dutch `1.250,50` is 1250.5. Values in slots follow the locale too: `message "Saved on {order.placed}"`.

## On each stack

Every stack shows the same texts and formats. On Rust, Go, C and C++ the web renderer carries the translations and fills in the formats; C# with Avalonia gets a `.resx` file per language and formats with .NET's culture data, pinned to the same answers.

See [Entities](data.entities.md) for fields, and [the language reference](../LANGUAGE.md#translations) for the full rules, with formats under [Formats](../LANGUAGE.md#formats).
