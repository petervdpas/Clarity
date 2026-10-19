# Clarity.Avalonia

[![NuGet](https://img.shields.io/nuget/v/Clarity.Avalonia.svg)](https://www.nuget.org/packages/Clarity.Avalonia/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Avalonia** is the user interface half of the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates, built on [Avalonia](https://avaloniaui.net/). Clarity is a designer-first IDE with its own small language: you design the windows, data and logic, and it writes readable source code for Rust, Go, C#, C or C++.

It stands on [Clarity.Runtime](https://www.nuget.org/packages/Clarity.Runtime/), which holds the data and logic.

## What it does

- **Clarity's own parts.** `ClarityTable` (a table whose cells can be edited: text, numbers, checkboxes, switches, choices, dates and moments), `ChoiceBox` (an enum's values or the records a reference can point at) and `DateTimeBox` (a day and a time on a 24-hour clock). They behave the same as Clarity's web parts; see below.
- **Windows.** `WindowViewModel` is the base of every generated window's view model: its handlers, its messages, and opening and closing windows. `Shell` shows the windows of an application, in place or in a desktop window of their own, and dialogs.
- **Languages and themes.** `Locale` shows the application in the person's language and formats, and lets them choose another; `Themes` switches the look.
- **Binding.** Converters for values in the person's format (`ShownConverter`, `EditConverter`, `FormatConverter`), choices for dropdowns (`Choice`) and state per part (`PartState`).

## How it is used

A Clarity project with `stack csharp avalonia desktop` generates a .NET project with a view and a view model per window. The views use these parts directly; a table with an edited column is written as

```xml
<c:ClarityTable x:Name="grid" ItemsSource="{Binding Tasks}" SelectedItem="{Binding Grid.SelectedItem}">
  <c:ClarityTable.Columns>
    <c:TableColumn Field="Title" Header="Task" Kind="Text" Edit="True" />
    <c:TableColumn Field="Done" Header="Done" Kind="Bool" Edit="True" Switch="True" />
  </c:ClarityTable.Columns>
</c:ClarityTable>
```

## The same on every renderer

A window must behave the same in a C# application as in a Rust, Go, C or C++ one, which Clarity draws with its web renderer. So every part is checked against cases shared with the web parts (`conformance/tables.json` and the others in the Clarity repository). A case says what the person does (tick a checkbox, type a time, press Enter, leave a field) and what must come out: which events run, which values change, what is marked. Both renderers run the same cases against their real controls; a difference fails the build.

## License

MIT, see [LICENSE.txt](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt).
