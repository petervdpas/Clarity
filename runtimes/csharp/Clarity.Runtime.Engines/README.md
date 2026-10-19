# Clarity.Runtime.Engines

[![NuGet](https://img.shields.io/nuget/v/Clarity.Runtime.Engines.svg)](https://www.nuget.org/packages/Clarity.Runtime.Engines/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Runtime.Engines** hosts the engines of the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates: processing modules (the logic of a game, document processing, a pricing engine) compiled once to WebAssembly, which every stack Clarity generates runs alike. It is built on [Wasmtime](https://www.nuget.org/packages/Wasmtime/).

An engine computes and nothing else: it imports nothing, so it reaches no database, file or network. The application hands it values and keeps what it gives back. An application references this package only when it has an engine, so the others do not carry Wasmtime.

## How it is used

Generated code loads each engine once from its embedded file and calls its functions:

```csharp
var pricing = Engine.Load("Pricing", bytes, ["add", "total"]);
long sum = pricing.Call<long>("add", 2, 40);
```

A module that is no engine, imports anything, or lacks a declared function is refused when it loads; an engine's own error arrives as a `RefusedException` in its words.
