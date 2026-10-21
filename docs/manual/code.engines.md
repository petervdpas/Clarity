# Engines

How to make, edit and build a processing engine compiled to WebAssembly in Clarity, call it from code, or bring one written in another language.

## What an engine is

An engine is a WebAssembly file that does one kind of processing: pricing, scheduling, a solver, the rules of a game. Every stack runs that same file, so the engine gives the same answers on every stack.

An engine only computes. It has no database, files, network or screen. Your code reads the records, hands them to the engine as values, and saves or shows what comes back. Work that needs the database while it runs belongs in an [action](code.functions.md), and the action can call an engine for the processing in between.

## Making an engine in Clarity

1. Click **+ New engine** under Engines in the project tree, and give it a name, such as `Pricing`. Clarity declares it with one function to start from, writes its code in Rust with a stub for that function, builds it, and opens it.
2. The engine opens in its own editor. On the left, its inspector holds what the declaration says:
   - **Functions**: name, values and what each gives. **+ Add function** adds one, and its stub appears in the code.
   - **Services**: clock, random, log and language.
   - **Texts it gives**: one a line, translated like the project's own; see [Texts and languages](#texts-and-languages).
   - **State**: none, per app or per window, and **On open**.
   - **Limits**: the time a call may run and the memory it may use.
3. On the right is the engine's code. Write what each function computes there, and save with Ctrl+S. Double-click a function in the inspector, or click its name above the code, to go to it.
4. Click **Build**, or just **Run**: Run and Build first build every engine whose code changed. The compiler's output goes to the Output panel, and its errors to Problems and to the lines of the code; click one to go there.

A line above the editor says how the engine stands, with the button that helps: **Ready**; the code changed since it was built (**Build**); the code lacks a function the inspector names (**Write the missing functions**); or what to install to build it here. **Text** shows the declaration as Clarity text instead of the inspector. Building needs Rust with its WebAssembly target (`rustup target add wasm32-unknown-unknown`); Clarity says so when it is missing.

An engine built elsewhere, in another language, is a file only: put it in the `engines` folder and declare it. Its editor edits the declaration and says whether the file fits it.

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
    uses clock, random
    limit 2 seconds, 64 MB
    function total(lines: list<Line>) -> money
    function size_of(count: integer) -> Size
    function discounted(order: Order) -> Order
```

- Every function gives a value, so each has `-> type`.
- Parameters and results are values (`text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`), `bytes`, enums, records and `list<Entity>`. Files are not: read the file in code and hand in its text (`read text of f`) or its bytes (`read bytes of f`).
- `uses` and `limit` are optional; see [Services](#services) and [Limits](#limits) below.
- The designer checks the file against the declaration, again whenever the `engines` folder changes. It reports a file that is not in the folder, is no WebAssembly, imports a service the engine does not use (or anything else), or lacks a function you declared.
- The project tree lists the engines under Engines, marking one that does not fit its file. Clicking one opens its editor; see [Making an engine in Clarity](#making-an-engine-in-clarity).

## Services

An engine reaches nothing by itself. `uses` names the services of Clarity it may call:

| Service | What the engine gets |
|---|---|
| `clock` | The time, in milliseconds since 1970-01-01 UTC. |
| `random` | A random number. Every call starts a fresh random sequence; the sequence is the same on every stack for the same start. |
| `log` | A line in the application's log, written as `engine Pricing: <the line>` to the standard error. |
| `language` | The language the person picked, as its tag such as `nl`; none for the texts as the project writes them. With `clarity-engine`, call `clarity_engine::language()`. |

There is no service for the database, files or the network, and there won't be: an engine that needs data gets it as a value from your code.

## Texts and languages

An engine that gives people something to read lists those texts with `says`:

```clarity
engine Board from "board.wasm"
    uses random
    says "Your move", "You win", "I win", "A draw"
    says "That square is taken"
```

The texts join the project's own, under i18n in the project tree, so each translation file gives them in its language. A text an engine function returns is shown in the person's language, and so is an error the engine gives back. An engine that needs the language itself, to sort or to format, uses `language`.

## Limits

Every call runs within limits, so an engine that loops without end, or grows without end, can't hang or exhaust your application:

- **Time:** a call that runs longer than its limit is stopped. Code gets an error such as `engine Pricing failed in total: it ran longer than 2 seconds`, which a handler shows like any error.
- **Memory:** the engine never gets more memory than its limit. Most engines then fail, with an error that names the engine and the function.

Write `limit 2 seconds`, `limit 64 MB`, or both: `limit 2 seconds, 64 MB`. Without a limit line, a call may run 5 seconds and the engine may use 256 MB. A time limit is between 1 and 3600 seconds; a memory limit between 1 and 4096 MB.

## State

Without a state line, every call starts the engine afresh: nothing it did in one call is there in the next. An engine that should remember says so:

```clarity
engine Board from "board.wasm"
    state per window
    on open
    function move(from: integer, to: integer) -> bool
    function score() -> integer
```

- `state per app` keeps one instance for the whole application, for something the engine loads once and keeps, such as a dictionary or a model.
- `state per window` keeps an instance for each open window: a game's board, a document being edited. It is made at the window's first call and goes when the window closes. Only a window's handlers call such an engine, each in its own window; a function or an action has no window to call it in.
- `on open` runs the engine's `clarity_open` whenever an instance is made: once for an engine with state, before every call for one without. It sets the engine up, for example the start of a game.
- A call that fails, or runs past its time limit, drops the instance: its memory may be half changed. The next call starts a new one and opens it again. A refusal keeps the instance, since the engine answered.
- There is no `on close`. An engine can't save or show anything, so when a window must keep what the engine holds, the window's own `on close` handler asks the engine and saves the answer.

## Calling an engine

Code calls an engine's function by the engine's name:

```clarity
function order_total(order: Order) -> money
    return Pricing.total(order.lines)

handler discountClicked
    order = Pricing.discounted(order)
    save order
```

- The checker types the call against the declaration, as it does a call to one of your functions.
- Handlers, functions and actions can all call an engine. A call has no effects, because an engine reaches nothing.
- A record that comes back is a copy. It is saved only when your code saves it.
- Without a state line, each call starts the engine afresh; see [State](#state).
- When the engine refuses (it gives back an error), code gets a refusal in the engine's words, as from `refuse`. A handler shows it like any other refusal.
- The application checks every engine when it starts, and stops there if a file does not fit its declaration, not at the first call.

## Writing an engine

The easiest way is to let Clarity write it (see [Making an engine in Clarity](#making-an-engine-in-clarity)): the code is Rust with the library `clarity-engine`, in a folder next to the file, `engines/pricing/` for `pricing.wasm`. The library writes the engine interface for you; a function takes its arguments as a tuple, in the declared order, and gives back anything serde writes:

```rust
clarity_engine::interface!();

clarity_engine::function!(total, |(lines,): (Vec<Line>,)| Ok(priced(&lines).to_string()));
clarity_engine::on_open!(|| Ok(()));
```

`Err` is the engine's refusal, in its words. `clarity_engine::clock()`, `random()` and `log(...)` are the services, and `clarity_engine::Bytes` is `bytes`. Clarity builds the engine as a `cdylib` for `wasm32-unknown-unknown` and copies the `.wasm` file into the `engines` folder; outside Clarity, `cargo build --release --target wasm32-unknown-unknown` in its folder does the same.

The examples have one each, with their source next to the file:

| Example | Engine | What it shows |
|---|---|---|
| shop | `Pricing` | an order's total from its lines, with volume discounts; called from a data event |
| records | `Images` | the size of a picture, read from its file's `bytes` |
| dial | `Gauge` | the needle's angle and colour, called from a draw handler |
| noughts | `Board` | a game kept with `state per window` and `on open`, using `random` |

Each opens in Clarity's engine editor with its code, and Run builds what changed. `examples/build-engines.sh` builds them all at once from a terminal.

### The engine interface

Any language that compiles to plain WebAssembly can make an engine, as long as the module imports nothing but the services it uses. Build for a target without an operating system: Rust's `wasm32-unknown-unknown`, clang's `--target=wasm32` without a C library, Zig's `wasm32-freestanding`. A target such as WASI adds imports of its own, and Clarity refuses the module.

The module exports:

| Export | What it does |
|---|---|
| `memory` | The engine's memory, which the host writes into and reads from. |
| `clarity_alloc(len: i32) -> i32` | Room for `len` bytes; gives its place. |
| `clarity_free(place: i32, len: i32)` | Gives back room the engine handed out. |
| `clarity_open(place: i32, len: i32) -> i64` | Only with `on open`: sets up a new instance, called like a function with no arguments. |
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
| `bytes` | a string with the bytes as base64, as `"AQID"` |

A function in Rust without the library, with `serde_json`:

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

The services are functions the module imports from module `clarity`:

| Import | Type |
|---|---|
| `clarity.clock` | `() -> i64` |
| `clarity.random` | `() -> i64` |
| `clarity.log` | `(place: i32, len: i32)`, the place and length of UTF-8 text in the engine's memory |

In Rust:

```rust
#[link(wasm_import_module = "clarity")]
unsafe extern "C" {
    fn clock() -> i64;
    fn random() -> i64;
    fn log(at: *const u8, len: i32);
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
- The service `progress`, which would let a long call show on the screen how far it is.
- Hooks such as `on tick`, for engines beside the screen.
- Engines that run beside the screen, drawing on a canvas.
