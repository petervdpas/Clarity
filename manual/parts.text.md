# Text parts

The parts of the Text group of the Parts palette: labels, help texts, headings, icons, badges, links, text and Markdown, which show words and take no input.

## Using them

Drop a text part from the **Parts** palette onto the window, then set what it shows in its properties. A part that shows a fixed text has a **Caption**, a key of the translations (`@nameLabel`); a part that shows a value has **Bind** (a field, as `customer.name`, or a plain value of the window, as `shown`). See [Parts](design.parts.md) for what every part has, and [Languages](languages.md) for keys.

A label, its input and the input's help text are three separate parts. **For input** in the properties of a label or a help text ties it to an input by the input's name:

```xml
<Form name="details" at="1,1" span="12" rows="3" bind="customer">
  <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
  <Textbox name="name" at="4,1" span="6" bind="customer.name" placeholder="@fullName" />
  <Help name="nameHelp" at="10,1" span="3" for="name" />
  <Label name="kindLabel" at="1,2" span="3" caption="@kindLabel" for="kind" />
  <Radio name="kind" at="4,2" span="6" bind="customer.kind" />
</Form>
```

A form can give its inputs their labels itself, in a label column; see [`form`](parts.containers.md#form).

### `label`

A caption for an input. Clicking it focuses its input. When the input is bound to a `required` field, the label shows a marker. A label is never bound to data; to show a value, use `text`.

```xml
<Label name="vatNumberLabel" at="1,2" span="3" caption="@vatNumberLabel" for="vatNumber" />
```

- `caption` (a key): the text it shows.
- `for` (a part name, none by default): the input it belongs to; **For input** in the properties lists the window's inputs.

The checker refuses a `for` that names no part of the window (`label at 1,2: there is no part 'vat' in this window`) or a part that is no input (`label at 1,2: 'save' is a button, not an input`), and a label `for` an input its form already labels (`the form labels 'nameBox' itself; remove this label`).

### `help`

A help text for an input. It shows its caption, or, when it has none, the `help` of the field its input is bound to (see [Entities](data.entities.md)). When a handler sets the input's `error`, the help text shows that message instead, and the input is marked; `name.error = ""` clears it (see [Handlers](code.handlers.md#part-properties)).

```xml
<Help name="vatNumberHelp" at="8,2" span="5" for="vatNumber" />
```

- `caption` (a key, none by default): a text of its own, in place of the field's help.
- `for` (a part name): the input it belongs to, with the same rules as a label's.

### `heading`

A heading, in the theme's heading font. It shows its caption, or with `bind` the value of a field of any type, as `bind="chapter.title"`.

```xml
<Heading name="coverHeading" at="1,1" span="10" caption="@coverTitle" bind="book.title" />
```

- `caption` (a key, "Heading" for a new one): the text it shows.
- `bind` (a field or plain value, none by default): a value it shows in place of its caption.

### `icon`

An icon of its own, one of the Lucide icons by name. A new one shows `star`; pick another under **Icon** in the properties.

```xml
<Icon name="warningIcon" at="1,1" icon="triangle-alert" />
```

- `icon` (an icon name): what it shows.

The checker refuses an icon Clarity does not know: `unknown icon 'triangle'; did you mean 'triangle-alert'?`.

### `badge`

A short text or number in a pill: its caption, or what it binds, a text, integer, decimal, money or enum value.

```xml
<Badge name="statusBadge" at="11,1" bind="order.status" />
```

- `caption` (a key, "New" for a new one): the text it shows when it binds nothing.
- `bind` (a field or plain value of type text, integer, decimal, money or enum).

### `link`

Text that opens something when clicked: an address written in `to`, the address it binds (a url, email or text field), or a handler (`onclick`). It does one of the three.

```xml
<Link name="iconsLink" at="1,4" span="3" caption="@iconsLink" to="https://lucide.dev" />
<Link name="websiteLink" at="4,4" span="3" bind="customer.website" />
<Link name="moreLink" at="7,4" span="2" caption="@moreLink" onclick="moreClicked" />
```

- `caption` (a key, "Link" for a new one): the text it shows.
- `to` (an address, none by default): written whole, starting with `https://`, `http://`, `mailto:` or `tel:`. **Opens** in the properties.
- `bind` (a url, email or text field).
- `taborder` (a whole number from 1): its place in the keyboard order.
- Event: `click`.

The checker refuses a link that does none of them (`a link opens an address ('to'), what it binds or runs a handler ('on click')`), one that does more than one (`a link opens an address, what it binds or runs a handler: one of them`), and a `to` that is no whole address (`'lucide.dev' is no address; write it whole, as in "https://lucide.dev"`).

### `text`

Shows the value of a field of any type, `bind="customer.name"`, or a plain value of the window, `bind="shown"`. On one row it is one line, cut short when it is too long. Over several rows (`rows="6"`) it is text to read: it wraps from the top, keeps its paragraphs (`\n\n` in a translation), and scrolls when it is longer than its rows.

```xml
<Text name="notesText" at="1,5" span="12" rows="6" bind="customer.notes" />
```

- `bind` (a field or plain value of any type): what it shows.

The checker refuses a `bind` to data the window does not have (`text at 1,5: unknown data 'client'`) or to a field its entity does not have (`text at 1,5: 'Customer' has no field 'note'`). A `text` part never shows a hashed field.

### `markdown`

Shows Markdown with its headings, lists, bold, italics, code and links, wrapped, scrolling when it is longer than the part. It shows one of three things:

- A file per language, by a key: `caption="@gameRules"` (**Key** in the properties), each language file pointing the key at its file in the project's `resources` folder, as `gameRules resource "rules-nl.md"` (see [Languages](languages.md#keys-that-point-at-resources)).
- One file for every language: `source="license.md"` (**Source**).
- A text it binds: `bind="notes"`, a text field or plain value.

A link to an `http`, `https` or `mailto` address opens in the browser. A link into the application, as `clarity:fragment/412`, runs its `link` event, and the handler reads where it pointed as `rules.link`.

```xml
<Markdown name="rules" at="1,1" span="6" rows="8" caption="@gameRules" />
<Markdown name="license" at="7,1" span="6" rows="8" source="license.md" />
```

- `caption` (a key), `source` (a file of the resources folder) or `bind` (a text field or plain value): what it shows, one of them.
- Event: `link`.

The checker refuses more than one of them: `a markdown shows a key, a file of the resources (source) or what it binds: one of them`.

## Related pages

- [Parts](design.parts.md): the palette, the properties panel, and what every part has.
- [Basic input parts](parts.input.md): the inputs labels and help texts belong to.
- [Containers](parts.containers.md): forms, panels and the other parts that hold parts.
- [Languages](languages.md): keys, translations and texts that point at resources.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Label | `label` | nothing | 3 x 1 | no | `for` | none |
| Help text | `help` | nothing | 4 x 1 | no | `for` | none |
| Heading | `heading` | field | 6 x 1 | no | none | none |
| Icon | `icon` | nothing | 1 x 1 | no | none | none |
| Badge | `badge` | field (text, integer, decimal, money, enum) | 1 x 1 | no | none | none |
| Link | `link` | field (url, email, text) | 2 x 1 | no | `to`, `tabOrder` | `click` |
| Text | `text` | field | 4 x 1 | no | none | none |
| Markdown | `markdown` | field (text) | 6 x 4 | yes | `source` | `link` |

<!-- /catalog -->
