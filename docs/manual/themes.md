# Themes

How to change the look of an application with themes: colors, fonts, sizes, part looks and named styles, and how people switch between themes.

## Clarity's own theme

Every application starts from Clarity's own look, the built-in theme `Clarity`. It sets every value: colors for light and dark mode, fonts, sizes, corner radii, spacing, borders, a shadow, and the look of each kind of part in its states. It follows the system's light or dark mode.

A theme of your own sets only what it changes; the rest stays Clarity's. Without any theme, the application looks as Clarity does, and parts cannot have a `style`.

## Writing a theme

A theme is declared in a module:

```clarity
theme Shop
    title "Shop"
    mode system
    color accent "#1f6f5c" dark "#4fb393"
    color brand "#f5c400"
    font "Inter"
    font heading "Georgia, serif"
    size 14
    radius 4
    radius button 12
    shadow flat
    part button hover background accent_soft
    part header background accent_soft, text accent
    style primary background accent, text on_accent, weight bold
```

- `title` is the name people see when they choose the theme. It is translated like other texts.
- `mode light`, `mode dark` or `mode system`.
- `color name "#rrggbb"`, with `dark "#rrggbb"` for its value in dark mode. Colors named after a role (`background`, `panel`, `text`, `muted`, `border`, `accent`, `on_accent`, `selection`, `error` and others) change the application itself; other names are the theme's own, for its parts and styles.
- `font`, `font heading`, `font mono`: lists of font families.
- `size`, `size small`, `size heading`: text sizes in pixels.
- `radius`, and `radius input`, `radius button` or `radius panel` for one kind; `padding`, `border`; `shadow none`, `flat` or `raised`.
- `part kind [state] look` changes a kind of part: `button`, `input`, `header` (of a table), `tab` or `menu`, optionally in the state `hover`, `pressed` or `focus`.
- `style name look` declares a named style.

A look sets `background`, `text` and `border` to a theme color by name or written out (a border can be `none`), `radius n`, and `weight bold` or `weight normal`, separated by commas.

A style also sets how its text shows: `size n` in pixels, `font text`, `font heading` or `font mono` (a font of the theme), and `align left`, `center` or `right`. Aligned text keeps an input's padding from the edge. A calculator's display, large and on the right in an LCD green:

```clarity
theme Calculator
    color lcd "#c7d3a8" dark "#1d2618"
    color lcd_text "#1f2a14" dark "#a8e890"
    style display background lcd, text lcd_text, border none, size 40, font mono, align right
```

and on the window: `text name display at 1,1 span 4 rows 2 bind shown style display`.

## Themes in the designer

Themes are listed under Themes in the project tree. Clarity's own theme opens read only, as the reference of every value. **New theme** adds a full copy of it to a module and opens it. A theme opens in its own view: its text, checked as you type, beside a sample window drawn with it.

## Offering themes

The project file names the themes the application offers; the first is the one it starts with:

```clarity
project Shop
    stack rust web desktop
    theme Shop Night
```

On the Project tab, under **Language, formats and theme**, you tick the themes to offer and pick the one it starts with. With more than one, **Show the designer in** shows the canvas in any of them.

To let people switch, give a menu item or button the click target `theme`:

```clarity
item "Theme" on click -> theme
```

It lists the themes by their titles, switches at once and remembers the choice for the next start.

## Styles on parts

A part picks a style by name, never raw colors:

```clarity
button "Save" at 9,6 span 2 style primary default on click: saveClicked
```

The **Style** field in the part properties (see [Parts](design.parts.md)) lists the styles of the project's theme. Every theme the application offers must have each style a part uses, so switching themes never drops one; a theme that misses it is an error.

## On each stack

- The web renderer (Rust, Go, C and C++) gets a CSS file of variables per theme: Clarity's own as `ui/themes/clarity.css`, each offered theme in its own file. The application holds the theme's class (`theme-shop`) and a styled part its style's class (`style-primary`). C and C++ put these styles inside the program's single page.
- C# with Avalonia gets one resource dictionary per offered theme, `Themes/Shop.axaml`, with Clarity's values merged in and light and dark colors. A style becomes a style class (`Classes="primary"`).

The designer draws with the web renderer, so for Avalonia the canvas shows the theme faithfully but not pixel for pixel; Run shows the real window. See [the language reference](../LANGUAGE.md#themes) for every line.
