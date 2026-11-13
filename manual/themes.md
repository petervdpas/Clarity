# Themes

How to change the look of an application with themes: colors, fonts, sizes, part looks and named styles, and how people switch between themes.

## Clarity's own theme

Every application starts from Clarity's own look, the built-in theme `Clarity`. It sets every value: colors for light and dark mode, fonts, sizes, corner radii, spacing, borders, a shadow, and the look of each kind of part in its states. It follows the system's light or dark mode, and looks like the Clarity IDE itself.

A theme says how the application looks as named values rather than a style sheet, so every stack can carry it. A theme of your own sets only what it changes; the rest stays Clarity's. Without any theme, the application looks as Clarity does, and parts cannot have a `style`.

## Themes in the designer

Themes are listed under Themes in the project tree. Clarity's own theme opens read only, as the reference of every value. **New theme** asks for the theme's name, its module, and what it starts as: light and dark as the system is (a full copy of Clarity's own), **Light**, **Dark**, or **Cream and green**. It writes every value a theme can set, so each is there to change, and opens the theme.

A theme opens in its own view with a sample window drawn with it, and two tabs. **Properties** shows every value a theme can set as a form: title, mode, padding, border and shadow; each color for light and dark mode with a color picker; the fonts, text sizes and corner radii; the look of each kind of part in each of its states; and the theme's styles. A value the theme does not set shows Clarity's own, greyed; changing it writes that one line of the theme, and **Reset** takes the line away again. **Add color** and **Add style** add the theme's own. **Text** is the theme's text, checked as you type.

The name at the top renames it, also where the project offers it; **Delete theme** removes it, and the application stops offering it. A right click on a theme in the project tree opens it or deletes it too, as it does for windows, entities, enums and engines.

## Offering themes

The project file names the themes the application offers; the first is the one it starts with:

```clarity
project Shop
    stack rust desktop
    theme Shop Night
```

On the Project tab, under **Language, formats and theme**, you tick the themes to offer and pick the one it starts with. With more than one, **Show the designer in** shows the canvas in any of them.

To let people switch, give a menu item or button the click target `theme` (see [The app and its frame](design.frame.md#built-in-click-targets)):

```xml
<Item caption="@themeItem" name="themeItem" opens="theme" />
```

It lists the themes by their titles, switches at once and remembers the choice for the next start.

The checker refuses a theme the project file names that no module declares.

## `theme`

A theme is declared in a module, one setting per line:

```text
// one setting per line
theme Name
    title @key
    mode light | dark | system
    color name "#rrggbb" [dark "#rrggbb"]
    font [text | heading | mono] "Families"
    size [text | small | heading] n
    radius [input | button | panel] n
    padding n
    border n
    shadow none | flat | raised
    part kind [state] look
    style name look
```

```clarity
theme Shop
    title @shopTheme
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
| `title @shopTheme` | The name people see when they choose the theme, a key (translated like other texts) or a text. Without it the theme's name shows, spaced out: `NightOwl` as "Night owl". |
| `mode light`, `mode dark`, `mode system` | The theme is always light, always dark, or follows the system. |
| `color name "#rrggbb"` | A color, written `#rrggbb` or `#rgb`; `dark "#rrggbb"` after it gives its value in dark mode. |
| `font "Families"`, `font heading ...`, `font mono ...` | The fonts of text, headings and code, as a list of families: `"Inter, sans-serif"`. |
| `size n`, `size small n`, `size heading n` | Text sizes in pixels: the text, small text (help, captions) and headings. |
| `radius n`, `radius input n`, `radius button n`, `radius panel n` | Corner radii in pixels: of everything, or of one kind. |
| `padding n` | The space inside inputs and buttons, in pixels. A card's padding and the gap between cards follow it. |
| `border n` | The width of borders, in pixels. |
| `shadow none`, `shadow flat`, `shadow raised` | The shadow under menus, popups and messages. |
| `part kind [state] look` | How a kind of part looks (see [Part looks](#part-looks)). |
| `style name look` | A named style that parts pick (see [Styles](#styles)). |

The checker refuses an unknown line ("a theme has title, mode, color, font, size, radius, padding, border, shadow, part and style"), an unknown mode, font, size, radius or shadow, a color named twice in one theme, and a color that is not `#rrggbb` or `#rgb`.

### Colors

Colors named after a role change the application itself; other names are the theme's own, for its parts and styles.

| Role | Used for |
|---|---|
| `background`, `panel`, `raised` | the window, panels and inputs, raised surfaces such as buttons and table headers |
| `frame`, `frame_text` | the menu bar, toolbar, sidebars and footer |
| `text`, `muted` | text, and quieter text such as help and labels |
| `border`, `strong_border` | lines between things, and the borders of inputs and buttons |
| `accent`, `accent_soft`, `on_accent` | what is chosen, active or important, a soft tint of it, and text on it |
| `selection`, `focus`, `input` | a chosen row, the focus ring, the inside of inputs |
| `error`, `warning`, `success` and their `_soft` tints | what went wrong, what needs care, what went well |
| `code_background`, `code_keyword`, `code_type`, `code_text`, `code_number`, `code_comment` | code in a memo: the background of code to read, and the colors of its words |
| `code_added`, `code_removed` | a diff in a memo: the lines it adds and those it takes out |

### Looks

A look (of a part or a style) is a list separated by commas:

| Word | Value |
|---|---|
| `background` | a color of the theme by name, a color written out, or `none` |
| `text` | a color of the theme by name, or written out |
| `border` | a color of the theme by name, a color written out, or `none` |
| `radius n` | the corner radius in pixels |
| `weight bold`, `weight normal` | the text's weight |
| `size n` | the text size in pixels, 6 to 200 |
| `font text`, `font heading`, `font mono` | a style only: a font of the theme |
| `align left`, `align center`, `align right` | a style only: how its text aligns; aligned text keeps an input's padding from the edge |

The checker refuses a color the theme does not have ("part button hover uses color 'x', which theme Shop does not have; its colors are ..."), `none` as a text color, a text size outside 6 to 200, an unknown weight or alignment, and `font` or `align` in a part's look ("font and align belong in a style").

### Part looks

`part kind [state] look` changes a kind of part:

| Kind | Is |
|---|---|
| `button` | a button |
| `input` | an input |
| `header` | the header of a table |
| `tab` | a tab |
| `menu` | a menu |
| `list` | a list, its frame and rows |
| `card` | a row of a list with **Cards** |
| `card_line` | the lines under a card's title |
| `code` | a read-only memo of code, so people see it is code to read, not a box to type in |

A state after the kind gives the look in that state: `hover`, `pressed` or `focus`, and `selected` for a list, a card or a card line (the row chosen) and a tab (the one open). A list with **Cards** shows each row as a `card`, its title in the card's text and weight, the lines under it as `card_line`:

```clarity
theme Cards
    part card background panel, border border, radius 8
    part card selected background accent, text on_accent
    part card_line text accent, weight bold
    part card_line selected text on_accent
```

The checker refuses another kind ("a theme sets the look of the parts button, input, header, tab, menu, list, card, card_line and code"), an unknown state, and `selected` on a kind that has none ("only list, card, card_line and tab have a selected look").

### Styles

`style name look` declares a named style. A style also sets how its text shows: `font` with a font of the theme and `align`. A calculator's display, large and on the right in an LCD green:

```clarity
theme Calculator
    color lcd "#c7d3a8" dark "#1d2618"
    color lcd_text "#1f2a14" dark "#a8e890"
    style display background lcd, text lcd_text, border none, size 40, font mono, align right
```

and on the window:

```xml
<Display name="screen" at="1,1" span="4" rows="2" bind="shown" style="display" />
```

## Styles on parts

A part picks a style by name, never raw colors:

```xml
<Button name="saveButton" at="9,6" span="2" caption="@saveCaption" style="primary" default="true" onclick="saveClicked" />
```

The **Style** field in the part properties (see [Parts](design.parts.md)) lists the styles of the project's theme. A window can have a style of its own too, `style="splashBack"` on its root, which fills the window and colors its parts' text (see [Windows](design.windows.md#a-pages-file)).

Every theme the application offers must have each style a part or window uses, so switching themes never drops one. The checker refuses a style when a theme the project offers misses it ("theme Night has no style 'primary'; its styles are ..."), and any style when the project file offers no theme ("style 'primary' comes from the project's theme; name it in the project file").

## On each stack

The generator writes each offered theme as Clarity resolves it, once for light and once for dark mode, as JSON files that Clarity.UI lays over Clarity's own theme on every stack and in the browser: `themes/` for Rust and C, `ui/themes/` for Go and C++, and `Themes/` for C#, where `AppThemes.cs` lists the themes offered, the one it starts with first. A file is named after the theme, `night-owl.light.json` and `night-owl.dark.json` for `theme NightOwl`. Without any theme, Clarity.UI carries Clarity's own.

The designer draws its canvas with CSS made from the same values, so the canvas shows the theme faithfully but not pixel for pixel; Run shows the real window.

## Related pages

- [The app and its frame](design.frame.md)
- [Windows](design.windows.md)
- [Parts](design.parts.md)
- [The project file](project.file.md)
