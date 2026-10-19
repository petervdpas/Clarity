# Clarity.Avalonia

The Avalonia half of the runtime of C# applications that Clarity generates, on top of `Clarity.Runtime`.

- Clarity's own parts, which behave the same as on Clarity's web renderer: `ClarityTable`, `ChoiceBox`, `DateTimeBox`
- The base of every window's view model, and opening and closing windows
- Languages, themes and messages

Each part is checked against the cases every Clarity renderer shares (`conformance/tables.json` and the others in the Clarity repository), so a window behaves the same in a C# desktop application as in a Rust, Go, C or C++ one.
