# Clarity.Ui

Clarity.UI for C#: Clarity's own UI framework, which draws its windows itself with Skia, as Avalonia does (DESIGN 7.9). Each stack has its version: Rust, Go, C, C++ and C#, alike in what they draw and reading the same window markup (`.cxaml`).

So far this package holds its model half: the base of every window's view model (`WindowViewModel`), the state of its parts (`PartState`), and what the runtime decides when the person acts: undo, reordering rows, ranges, trees, charts, table pages, dropdown options, questions and messages. `Clarity.Avalonia`, the alternative and the reference, draws the same model with Avalonia.
