# Basic input parts

The parts of the Basic input group of the Parts palette: text boxes, memos, numbers, check boxes, dropdowns, dates, files, colors, sliders and the other inputs, the button, and the sign-in part of the splash.

## Using them

Drop an input from the **Parts** palette onto the window, then pick what it binds under **Bind** in its properties: the list offers only the fields and plain values of the types that input takes (see [Parts](design.parts.md#binding)). What the person types goes into the data, and handlers read and change the data by name. An input that is not bound yet is allowed while you design; a text box, memo or password input that binds nothing holds its own text, which code reads and sets as `name.text`, and a check box, switch or toggle its own `checked`.

Give each input a label and, where it helps, a help text: separate parts tied to it with `for` (see [Text parts](parts.text.md#label)).

### What inputs have in common

- `placeholder` (a key, none by default): shown in a muted color while the input is empty: nothing typed, no day, no color, no tag, nothing chosen. **Placeholder** in the properties. Text boxes, memos, password inputs, numbers, spin boxes, folders, dates, colors, tags and dropdowns have it. Tags without one say Clarity's word `addTag`.
- `readonly` (true or false, false by default): shows the value, which cannot be changed. Handlers can change this, as `name.readonly = true`. **Read-only** under Behavior.
- `noUndo` (true or false, false by default): Ctrl+Z takes back nothing on this part, not even typing in a text box. Every other input takes back its own edits. **Undo (Ctrl+Z)** under Behavior, ticked by default.
- `taborder` (a whole number from 1, none by default): its place in the keyboard order. Parts without one follow in reading order. **Tab order** in the properties; the designer refuses 0.

Code sets an input's `error` to mark it and show a message in its help text, and `""` clears it; see [Handlers](code.handlers.md#part-properties).

### When `change` runs

`enter` and `leave` run when the focus moves into and out of an input. `change` runs when the person changes an input's value to one that differs, and only then, on every stack:

- Typed text is taken when the input is left or Enter is pressed; typing alone is no change, and neither is leaving without typing or typing the same text again. In a `memo`, Enter is a new line and the text is taken when it is left.
- A number reads in the person's format (see [Languages](languages.md#formats)). Text that is no number stays as typed and is marked, and the value stays; a cleared number is zero.
- A check box, switch, toggle, dropdown, radio group or date changes when the person ticks or picks another value.
- What a handler sets, and a record read again, is never a change.
- Enter in an input of a `form` takes the input first, then runs the form's `submit` (see [`form`](parts.containers.md#form)).

### `textbox`

One line of text, bound to a text, email or url field. With `copy` a button in its corner puts its text on the clipboard, also while it is read-only; Clarity's word `copyText` names that button.

```xml
<Textbox name="email" at="4,2" span="6" bind="customer.email" placeholder="@emailPlaceholder" onchange="emailChanged" />
```

- `bind` (a text, email or url field or plain value).
- `copy` (true or false, false by default): **Copy button**.
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Events: `change`, `enter`, `leave`.

Code reads where the person types: `email.caret` (how many characters stand before the caret, set too) and `email.selectedText`; `email.insert(text)` puts text where the selection is, as one step Ctrl+Z takes back.

The checker refuses a field of another type: `textbox at 4,2: 'age' is integer; a textbox shows text, email, url`.

### `memo`

Text over several lines, bound to a text field. Enter starts a new line, and the text is taken when the memo is left.

- `code` (one of `clarity`, `csharp`, `rust`, `go`, `c`, `cpp`, `java`, `javascript`, `typescript`, `python`, `sql`, `json`, `xml`, `html`, `css`, `yaml`, `markdown`, `shell`, `diff`; none by default): the text is code, in the mono font, its comments, texts, numbers, keywords and type names in the theme's colors (`code_keyword`, `code_type`, `code_text`, `code_number`, `code_comment`); a `diff` colors its added and removed lines with `code_added` and `code_removed`. Read-only, it takes the theme's `code` part look, its own background (`code_background`) and a quieter border. **Code** in the properties.
- `markdown` (true or false, false by default): a Markdown editor with a preview: Write and Preview on its top, the text colored as Markdown while it is written, Ctrl+B and Ctrl+I making the selection bold and italic. In the preview a link into the application (`clarity:fragment/412`) runs its `link` event, and the handler reads where it pointed as `manuscriptBox.link`. **Markdown** in the properties, off while **Code** names a language.
- `toolbox` and `toolboxPage`: a toolbox page of coded tools that work on the text where the person is (`toolboxPage="WritingTools"`). It comes at the caret (`toolbox="caret"`: Ctrl+Space calls it up under the caret, a right-click where it was pressed, with the memo's Cut, Copy and Paste as its last row), floats over the window (`popup`), or lies against the memo's edge (`slideout`). See [Toolbox parts](parts.toolbox.md).
- `copy`, `placeholder`, `readonly`, `noUndo`, `taborder`: as a text box's.
- Events: `change`, `enter`, `leave`, `link`.

```xml
<Memo name="manuscriptBox" at="1,1" span="12" rows="10" bind="chapter.text" markdown="true" toolbox="caret" toolboxPage="WritingTools" onchange="textChanged" />
<Memo name="sourceBox" at="1,11" span="12" rows="6" bind="snippet.code" code="csharp" readonly="true" copy="true" />
```

A memo reads and sets `caret`, reads `selectedText` and takes `insert(text)` as a text box does, so a tool can turn the selection into a quote: `manuscriptBox.insert("> " + manuscriptBox.selectedText)`. It also takes rows dragged onto it from a list, table or tree (**Takes rows from** in the properties): the caret goes where the row was let go, then the handler runs. A memo's toolbox holds tools with `onclick`, buttons and labels; the checker refuses one with painting tools (`WritingTools works on a canvas's picture; a memo's toolbox holds tools with 'on click', buttons and labels`).

### `password`

Text hidden while it is typed, bound to a text or a `secret` field. The eye beside it shows what is typed or hides it again, and the button next to it copies it. It is the one input that shows a secret, as dots.

```xml
<Password name="apiKey" at="4,3" span="6" bind="settings.apiKey" placeholder="@apiKeyPlaceholder" />
```

- `bind` (a text or secret field or plain value).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Events: `change`, `enter`, `leave`.

The checker refuses a secret field on any other input: `textbox at 4,3: 'apiKey' is secret; only a password input shows it, as dots`.

### `signin`

Signs in with the project's strategies (see [Signing in](project.signin.md)): per strategy its inputs and button, in Clarity's own words, and a refused try inside itself. It keeps only its place and look: what it offers is set under **Who is using it** in Project settings (the `signin` block of the project file). It binds nothing and has no events.

```xml
<Splash class="shop.Welcome">
  <Signin name="signIn" at="3,4" span="8" rows="6" />
</Splash>
```

The checker refuses a sign-in part without `signin` in the project file (`a signin part signs in, which needs signin in the project file, like: signin Member`), one anywhere but on the app's splash (`a signin part stands on the app's splash; the sign-in window is designed from its views`), and a second one in the project (`there is already a signin part, on Welcome; a project has one`). The sign-in window is built from the parts of the Sign-in group instead; see [Sign-in parts](parts.signin.md).

### `number`

A number typed in the person's format, bound to an integer, decimal, money, id or duration field.

```xml
<Number name="quantity" at="4,4" span="2" bind="line.quantity" onchange="quantityChanged" />
```

- `bind` (an integer, decimal, money, id or duration field or plain value). A plain `decimal` shows the digits it has.
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Events: `change`, `enter`, `leave`.

### `checkbox`

A box ticked or not, bound to a bool. It labels itself with its caption, or without one with its field's label.

```xml
<Checkbox name="vinylBox" at="1,1" span="4" caption="@onlyVinylCaption" bind="onlyVinyl" onchange="refilter" />
```

- `caption` (a key, "Check box" for a new one).
- `bind` (a bool field or plain value), or none: it holds its own `checked`.
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `toggle`

A button that stays pressed: bound to a bool as a check box is, or holding its own, which code reads and sets as `boldToggle.checked`. Its `change` runs when the person presses it. Toggles joined in one row are a [button group](parts.containers.md#buttons).

```xml
<Toggle name="boldToggle" at="1,1" span="2" caption="@boldCaption" icon="bold" bind="note.bold" onchange="restyle" />
```

- `caption` (a key, "Toggle" for a new one).
- `icon` (an icon name, none by default): shown beside its caption.
- `bind` (a bool field or plain value), or none.
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `switch`

An on and off switch, bound to a bool. It labels itself as a check box does.

```xml
<Switch name="activeSwitch" at="4,5" span="3" caption="@activeCaption" bind="customer.active" />
```

- `caption` (a key, "Switch" for a new one).
- `bind` (a bool field or plain value), or none.
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `dropdown`

A choice from a list, bound to an enum, a reference (`ref Customer`), a language or theme setting (the project's languages, the offered themes), or text with `typed`. An enum offers no empty choice; a reference offers one first, for no record.

- `typed` (true or false, false by default): a dropdown of a text field that takes any text, and offers the values its field already has as it is typed. Each value is offered once, sorted ignoring case; typing filters them. Picking one or leaving the box with new text is the change. **Typed** in the properties.
- `search` (true or false, false by default): picks a reference among many records. Typing filters them, a record staying when every word typed is in its text or in a field it shows. Emptying the box picks no record. Every stack offers and filters alike. **Search** in the properties.
- `show` (field names, none by default): with `search`, the fields each record shows beside its text. **Shown fields** in the properties.
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

```xml
<Dropdown name="cityBox" at="1,1" span="4" bind="customer.city" typed="true" />
<Dropdown name="customerBox" at="5,1" span="6" bind="order.customer" search="true" show="name, city" />
```

The checker refuses:

- `typed` on a field that is not text (`'typed' takes any text, so it binds a text field`), and a text field without it (`a dropdown of text is 'typed': any text may be typed, the values it has are offered`).
- `search` on a field that is no reference (`'search' finds a record, so it binds a reference`), `typed` and `search` together (`a dropdown is 'typed' or 'search', not both`), and `show` without `search` (`'show' needs 'search': write 'search show name, city'`).
- A shown field the record does not have (`Customer has no field 'town' to show`) or a picture, file or list (`a search shows text and values, not photo`).

### `radio`

Radio buttons, one per value of an enum.

```xml
<Radio name="kind" at="4,2" span="6" bind="customer.kind" orientation="vertical" />
```

- `bind` (an enum field or plain value).
- `orientation` (`horizontal` or `vertical`): side by side or stacked; **Orientation** in the properties.
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `date`

A day, or a day with a time, bound to a date or datetime field.

```xml
<Date name="dueDate" at="4,6" span="3" bind="order.due" placeholder="@noDueDate" />
```

- `bind` (a date or datetime field or plain value).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `file`

A file of a record, bound to a `file` field: its name, with buttons to choose, open, save a copy of and remove it. See [Pictures and files](data.pictures-files.md).

```xml
<File name="contract" at="4,7" span="6" bind="customer.contract" />
```

- `bind` (a file field).
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `folder`

A folder setting's path (`bind="settings.photosFolder"`): the folder a file store keeps its files in, typed, or chosen with the system's folder picker through its button. In a browser the path is typed.

```xml
<Folder name="photosFolder" at="4,2" span="6" bind="settings.photosFolder" />
```

- `bind` (a folder setting).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Events: `change`, `enter`, `leave`.

### `tags`

A list of values as chips, bound to a list of text or of an enum's values (`list<text>`, `list<Genre>`): an item typed or an enum's value picked is added, and a chip is removed again.

```xml
<Tags name="genres" at="4,3" span="6" bind="album.genres" />
```

- `bind` (a list of values).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `color`

A color, bound to a `color` field: a swatch and the color as `#rrggbb`. The swatch opens a palette of 60 colors, the same on every stack. A color can also be typed, with or without `#`, in three or six digits (`f0a` is `#ff00aa`). Text that is no color stays as typed and is marked, and the field keeps its color. Tables show a color column with its swatch and edit it with the same input.

```xml
<Color name="labelColor" at="4,4" span="3" bind="category.color" />
```

- `bind` (a color field or plain value).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

### `slider`

A number between a lowest and a highest value, dragged along a bar, bound to an integer, decimal or money field.

```xml
<Slider name="discountSlider" at="1,1" span="6" bind="line.discount" min="0" max="50" step="0.5" onchange="discountMoved" />
```

- `min` and `max` (numbers): its lowest and highest value. Without them it takes the field's own `min` and `max` (see [Entities](data.entities.md)), and without those it runs from 0 to 100. **Lowest** and **Highest** in the properties.
- `step` (a number, 1 by default): how far it moves at a time.
- `readonly`, `noUndo`, `taborder`: see above.
- Event: `change`.

The arrow keys move a step, Page Up and Page Down ten, Home and End go to the ends. It takes the nearest step where it is let go, and runs its `change` then.

The checker refuses `min` not below `max` (`'min' is below 'max': 50 is not below 10`), a step of 0 or less (`'step' is a number above 0`), and a step that is no whole number on an integer field (`'quantity' holds whole numbers: the step is a whole number`).

### `spinbox`

A number typed, or moved a step at a time with its up and down arrows, bound to an integer, decimal or money field.

```xml
<Spinbox name="quantityBox" at="7,1" span="2" bind="line.quantity" min="1" />
```

- `min` and `max` (numbers): its range. Without them it takes the field's own `min` and `max`, and without those it goes on without end. A number typed is kept within its range.
- `step` (a number, 1 by default).
- `placeholder`, `readonly`, `noUndo`, `taborder`: see above.
- Events: `change`, `enter`, `leave`.

The arrow keys move a step, Page Up and Page Down ten. Code reads and sets its range as whole numbers, `levelBox.min` and `levelBox.max`: as the window writes them (0 and 2147483647 where it writes none) until a handler sets them, as a dial that goes up to the newest version does with `levelBox.max = top`. The checker's rules for the range are a slider's.

### `button`

A button with a caption and, if you like, an icon. It binds nothing; a click runs its handler (`onclick`) or opens something (`opens`, see [Clicking to open a window](design.parts.md#clicking-to-open-a-window)), not both.

```xml
<Button name="saveButton" at="11,10" span="2" caption="@saveCaption" icon="save" key="Ctrl+S" default="true" onclick="saveClicked" />
<Button name="sevenKey" at="1,3" caption="@sevenCaption" key="7" onclick="digit(7)" />
```

- `caption` (a key, "Button" for a new one).
- `icon` (an icon name, none by default): shown beside its caption; **Icon** in the properties.
- `key` (a key, none by default): the key that clicks it, anywhere in the window; **Shortcut** in the properties.
- `default` (true or false, false by default): Enter clicks it. **Enter clicks it (default)** under Behavior.
- `cancel` (true or false, false by default): Escape clicks it. **Escape clicks it (cancel)** under Behavior.
- `split` (true or false, false by default): an arrow beside it opens its menu, a `<Menu>` inside it written as a context menu is; the button itself runs its click. **Split** in the properties, then **Add its menu**.
- `taborder`: see above.
- Event: `click`. With values, `onclick="digit(7)"` or `onclick="operation(Add)"`, it runs the handler with them, one per parameter: a number, text in quotes, `true`, `false` or a value of the parameter's enum. See [Handlers](code.handlers.md#values-from-a-click).

A key is written as modifiers, then one key, joined by `+`: `"Ctrl+S"`, `"Ctrl+Shift+F2"`, `"Alt+Delete"`. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. The key is a letter, a digit, one of `+ - * / . , =`, `F1` to `F12`, or one of `Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`; `"Ctrl++"` is Ctrl and `+`. A key without Ctrl, Alt or Meta that is a character (`"7"`, `"+"`) is what is typed, from the keyboard or the number pad, on any keyboard layout; while the person types in a text input it is their typing and presses nothing. The designer refuses anything else (`'Ctrl+Hyper' is not a shortcut, as in 'Ctrl+S'`). How a button's key goes before the frame's is in [The app and its frame](design.frame.md#keys).

```xml
<Button name="saveButton" at="7,1" span="2" caption="@saveCaption" split="true" onclick="save">
  <Menu>
    <Item name="saveAsItem" caption="@saveAsItem" onclick="saveAs" />
  </Menu>
</Button>
```

The checker refuses a second default or cancel button in a window (`more than one default button in this window`, `more than one cancel button in this window`), a split button without its menu (`a split button opens its menu: write a 'menu' block under it`) or without its own click (`a split button runs its click too: wire 'on click'`), and a key that runs two different commands among a window's buttons.

## Related pages

- [Parts](design.parts.md): the palette, the properties panel, binding, and clicking to open a window.
- [Text parts](parts.text.md): labels and help texts for inputs.
- [Containers](parts.containers.md): forms, panels, tabs and button groups.
- [Handlers](code.handlers.md): the code behind, events and part properties.
- [Entities and enums](data.entities.md): the field types inputs bind to.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Text box | `textbox` | field (text, email, url) | 4 x 1 | no | `placeholder`, `copy`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave` |
| Memo | `memo` | field (text) | 6 x 3 | yes | `placeholder`, `code`, `markdown`, `toolbox`, `copy`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave`, `link` |
| Password | `password` | field (text, secret) | 4 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave` |
| Sign-in | `signin` | nothing | 8 x 6 | no | none | none |
| Number | `number` | field (integer, decimal, money, id, duration) | 3 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave` |
| Check box | `checkbox` | field (bool) | 3 x 1 | no | `readonly`, `noUndo`, `tabOrder` | `change` |
| Toggle button | `toggle` | field (bool) | 2 x 1 | no | `readonly`, `noUndo`, `tabOrder`, `icon` | `change` |
| Switch | `switch` | field (bool) | 3 x 1 | no | `readonly`, `noUndo`, `tabOrder` | `change` |
| Dropdown | `dropdown` | field (enum, ref, language, theme, text) | 4 x 1 | no | `typed`, `search`, `show`, `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change` |
| Radio buttons | `radio` | field (enum) | 6 x 1 | no | `orientation`, `readonly`, `noUndo`, `tabOrder` | `change` |
| Date | `date` | field (date, dateTime) | 3 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change` |
| File | `file` | field (file) | 6 x 1 | no | `readonly`, `noUndo`, `tabOrder` | `change` |
| Folder | `folder` | field (folder) | 6 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave` |
| Tags | `tags` | field (values) | 6 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change` |
| Color | `color` | field (color) | 3 x 1 | no | `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change` |
| Slider | `slider` | field (integer, decimal, money) | 6 x 1 | no | `min`, `max`, `step`, `readonly`, `noUndo`, `tabOrder` | `change` |
| Spin box | `spinbox` | field (integer, decimal, money) | 3 x 1 | no | `min`, `max`, `step`, `placeholder`, `readonly`, `noUndo`, `tabOrder` | `change`, `enter`, `leave` |
| Button | `button` | nothing | 2 x 1 | no | `tabOrder`, `shortcut`, `default`, `cancel`, `icon`, `split` | `click` |

<!-- /catalog -->
