# Clarity.Runtime.Engines

[![NuGet](https://img.shields.io/nuget/v/Clarity.Runtime.Engines.svg)](https://www.nuget.org/packages/Clarity.Runtime.Engines/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Runtime.Engines** hosts the engines of the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates: processing modules (the logic of a game, document processing, a pricing engine) compiled once to WebAssembly, which every stack Clarity generates runs alike. It is built on [Wasmtime](https://www.nuget.org/packages/Wasmtime/).

An engine computes and nothing else: it imports only the services of Clarity it declares (the clock, random numbers, a log), so it reaches no database, file or network. Every call runs within the engine's limits: one that runs too long is stopped, and the engine never gets more memory than its limit. The application hands it values and keeps what it gives back. An application references this package only when it has an engine, so the others do not carry Wasmtime.

## How it is used

Generated code loads each engine once from its embedded file and calls its functions:

```csharp
var pricing = Engine.Load("Pricing", bytes, ["add", "total"], ["clock"], new EngineLimits(Seconds: 2, Megabytes: 64));
long sum = pricing.Call<long>("add", 2, 40);
```

A module that is no engine, imports anything but the services it uses, or lacks a declared function is refused when it loads; a call that runs past its time limit fails with an `EngineException`; an engine's own error arrives as a `RefusedException` in its words.
