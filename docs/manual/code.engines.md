# Engines

How to use a processing engine compiled to WebAssembly: declaring it with its file, calling it from code, and writing one in Rust, C or Zig.

## What an engine is

An engine is a WebAssembly file that does one kind of processing: pricing, scheduling, a solver, the rules of a game. Every stack runs that same file, so the engine gives the same answers on every stack.

An engine only computes. It has no database, files, network or screen. Your code reads the records, hands them to the engine as values, and saves or shows what comes back. Work that needs the database while it runs belongs in an [action](code.functions.md), and the action can call an engine for the processing in between.

## Declaring an engine

Put the file in the `engines` folder of the module folder, next to `images`:

```text
shop.clarity
shop/
    sales.clarity
    engines/
        pricing.wasm
```

Then declare it in a module, with each function it has:

```clarity
engine Pricing from "pricing.wasm"
    function total(lines: list<Line>) -> money
    function size_of(count: integer) -> Size
    function discounted(order: Order) -> Order
```

- Every function gives a value, so each has `-> type`.
- Parameters and results are values (`text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`), enums, records and `list<Entity>`. Files are not: read the file in code and hand its text in.
- The designer checks the file against the declaration. It reports a file that is not in the folder, is no WebAssembly, imports anything, or lacks a function you declared.

## Calling an engine

Code calls an engine's function by the engine's name:

```clarity
function order_total(order: Order) -> money
    return Pricing.total(order.lines)

handler discount_clicked
    order = Pricing.discounted(order)
    save order
```

- The checker types the call against the declaration, as it does a call to one of your functions.
- Handlers, functions and actions can all call an engine. A call has no effects, because an engine reaches nothing.
- A record that comes back is a copy. It is saved only when your code saves it.
- Each call starts the engine afresh. Nothing carries over from one call to the next.
- When the engine refuses (it gives back an error), code gets a refusal in the engine's words, as from `refuse`. A handler shows it like any other refusal.
- The application checks every engine when it starts, and stops there if a file does not fit its declaration, not at the first call.

## Writing an engine

Any language that compiles to plain WebAssembly can make an engine, as long as the module imports nothing. Build for a target without an operating system: Rust's `wasm32-unknown-unknown`, clang's `--target=wasm32` without a C library, Zig's `wasm32-freestanding`. A target such as WASI adds imports, and Clarity refuses the module.

The module exports:

| Export | What it does |
|---|---|
| `memory` | The engine's memory, which the host writes into and reads from. |
| `clarity_alloc(len: i32) -> i32` | Room for `len` bytes; gives its place. |
| `clarity_free(place: i32, len: i32)` | Gives back room the engine handed out. |
| each function, by its name | `(place: i32, len: i32) -> i64` |

A call goes like this:

1. The host writes the arguments as a JSON array (UTF-8) into room from `clarity_alloc`, and calls the function with its place and length. The engine takes that room over.
2. The function writes its answer as JSON: `{"value": ...}`, or `{"error": "why"}` to refuse.
3. It returns the answer's place and length packed into one i64: the place in the high 32 bits, the length in the low 32.
4. The host reads the answer and gives its room back with `clarity_free`.

Values travel as Clarity's JSON:

| Type | In JSON |
|---|---|
| `text` | a string |
| `integer`, `duration` | a number (a duration in seconds) |
| `decimal`, `money` | a string with the exact number, as `"29.85"` |
| `bool` | `true` or `false` |
| `date`, `datetime` | a string, as `"2026-10-20"` and `"2026-10-20T14:30:00"`; `null` when empty |
| an enum | the value's name, as `"Large"` |
| a record | an object with its `id` and its fields, by their names |
| `list<Entity>` | an array of those objects |

A function in Rust, with `serde_json`:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn add(at: i32, len: i32) -> i64 {
    let bytes = unsafe { Vec::from_raw_parts(at as *mut u8, len as usize, len as usize) };
    let args: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap_or_default();
    let sum = args[0].as_i64().unwrap_or(0) + args[1].as_i64().unwrap_or(0);
    let answer = serde_json::json!({ "value": sum }).to_string().into_bytes().into_boxed_slice();
    let (place, size) = (answer.as_ptr() as i64, answer.len() as i64);
    std::mem::forget(answer);
    (place << 32) | size
}
```

The test engine in Clarity's repository, `conformance/engines/test`, is a complete example with `clarity_alloc` and `clarity_free`. Build it with `cargo build --release --target wasm32-unknown-unknown`.

## How each stack runs it

Each stack hosts the file with a WebAssembly runtime written for it. You install nothing extra.

| Stack | Runs engines with | Comes from |
|---|---|---|
| Rust | wasmi | the `engines` feature of the crate `clarity-runtime`, turned on only when the project has an engine |
| Go | wazero | the package `engine` of the runtime module |
| C# | Wasmtime | the package `Clarity.Runtime.Engines`, referenced only when the project has an engine |
| C and C++ | wasm3 | copied into `clarity-runtime/` with the engine host, only when the project has an engine |

The generated program carries the engine inside it: embedded on Rust, Go and C#, as bytes in `src/engines.c` or `src/engines.cpp` on C and C++. Shared tests (`conformance/engines.json`) run the same calls through every host.

## Not yet

These are designed but not built:

- Engines written in Clarity itself.
- Services an engine may ask for: the clock, random numbers, a log and progress.
- Time and memory limits per call. Until then, an engine that never finishes holds up the handler that called it.
- Engines that keep state per window or per application, and hooks such as `on tick`.
- Engines that run beside the screen, drawing on a canvas.
