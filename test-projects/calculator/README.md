# Calculator

A pocket calculator: a display and a keypad in one small window. It adds,
subtracts, multiplies and divides, and rounds each result to the number of
decimals you set in Preferences.

## What you see and can do

The Main window holds one view, the Keypad: a 12-digit display on top and
the keys below it.

- Digits, the point and the four operators, each also on its keyboard key (`7`, `+`, `/`, `=` and so on). Enter is the default key and gives the result.
- **C** (or Escape) clears everything, **+/-** changes the sign and **%** divides the entry by 100.
- An operator works out what is pending before it takes the next number, so `2 + 3 ×` shows 5.
- Dividing by zero is refused with a message (`refuse @cannotDivideByZero`).

The app's menu holds Preferences and Quit (Ctrl+Q). Preferences is a dialog
with the language, the theme and the decimals (0 to 8, default 4). These are
settings: each person keeps their own on their machine.

The display has the look of an LCD screen. The Calculator theme defines its
colors, light and dark, and gives the operator and equals keys their own
style. The theme follows the system's light or dark mode.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- A view's own data: the keypad keeps its state (`shown`, `entry`, `total`, `pending`, `factor`, `fresh`, `digits`) as `Data` in `Keypad.cxaml`, not in the database.
- A plain function, `calculate`, called from the handlers.
- Settings with a range and a default, edited in a form bound to `settings`.
- A theme with colors of its own and named styles for parts.

## Files

- `../calculator.clarity`: the project file (stack, output folder, the SQLite data source, the theme).
- `calculator.clarity`: the settings, the Calculator theme, the Operation enum, `calculate` and the Keypad's handlers.
- `design/`: `Calculator.cxaml` (the app and its menu), `Main.cxaml` (the window holding the keypad), `Keypad.cxaml` (the display and keys), `Preferences.cxaml` (the dialog).
- `translations/`: English and Dutch.
- `migrations/main/`: two steps. The first added a Calc entity for the calculator's state; the second removed it when the state moved into the window.
