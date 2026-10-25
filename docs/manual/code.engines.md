# Engines

How to write a processing engine in Clarity, or make, edit and build one compiled to WebAssembly, call it from code, or bring one written in another language.

## What an engine is

An engine does one kind of processing: pricing, scheduling, a solver, the rules of a game. There are two kinds, called alike from code:

- **Written in Clarity.** The engine is Clarity code in a module, and every stack generates it with the application, as it does your functions. It needs no other language and nothing to install. See [An engine written in Clarity](#an-engine-written-in-clarity).
- **Compiled to WebAssembly.** The engine is a file in the `engines` folder, written in Rust or any language that builds to WebAssembly. Every stack runs that same file, so it gives the same answers on every stack.

An engine only computes. It has no database, files, network or screen. Your code reads the records, hands them to the engine as values, and saves or shows what comes back. Work that needs the database while it runs belongs in an [action](code.functions.md), and the action can call an engine for the processing in between.

## An engine written in Clarity

Click **+ New engine** under Engines in the project tree, give it a name, and pick Clarity under **Written in** (the dialog starts with the choice you made last time). Clarity writes it with one function to start from and opens it. The project tree says what each engine is written in, beside its functions: `Board  Clarity, 8 functions`, `Pricing  WebAssembly, 3 functions`.

```clarity
engine Dice
    uses random
    state per app
    keep rolled: integer = 0

    function roll(sides: integer) -> integer
        rolled = rolled + 1
        return random(sides)

    function rolls() -> integer
        return rolled
```

- On the right is the engine as Clarity code: what it keeps and its functions. Save with Ctrl+S; the checker marks a problem on its line as you type. **Fill** gives the code the whole editor.
- On the left, the inspector lists its functions (double-click one to go to it) and what it keeps, and sets its **Services** and **State**: it writes the `uses` and `state` lines into the code.
- **`keep name: type = value`** is a value the engine keeps from one call to the next, with what it starts as. A keep is a plain value or a list of them, such as `list<integer>`, with items counted from 1: `squares[3]`.
- **State** decides whose values they are: without a state line every call starts from the values as written; `state per app` keeps one set for the whole application; `state per window` keeps a set for each open window, which goes when the window closes.
- **A call that fails changes nothing.** A function works on a copy of what the engine keeps and keeps it only when it succeeds, so a refusal leaves the values as they were.
- **It keeps to itself**: no windows, data, parts, messages or database. It gives back what the screen should show, and its texts are keys, translated like your own.
- **`random(n)`**, with `uses random`, gives a whole number from 1 to `n`, from the same sequence every stack gives (`conformance/lists.json`).

The noughts example keeps its game in such an engine: `Board`, with the board as `keep squares: list<integer>`, one board for each window.

## Making an engine in WebAssembly

1. Click **+ New engine** under Engines in the project tree, give it a name, such as `Pricing`, and pick **Rust, built to WebAssembly** under **Written in**. Clarity declares it with one function to start from, writes its code in Rust with a stub for that function, builds it, and opens it.
2. The engine opens in its own editor. On the left, its inspector holds what the declaration says:
   - **Functions**: name, values and what each gives. **+ Add function** adds one, and its stub appears in the code.
   - **Services**: clock, random, log and language.
   - **Keys it says**: one a line, the keys of the texts it gives; see [Texts and languages](#texts-and-languages).
   - **State**: none, per app or per window, and **On open**.
   - **Limits**: the time a call may run and the memory it may use.
3. On the right is the engine's code. Write what each function computes there, and save with Ctrl+S. Double-click a function in the inspector, or click its name above the code, to go to it.
4. Click **Build**, or just **Run**: Run and Build first build every engine whose code changed. The compiler's output goes to the Output panel, and its errors to Problems and to the lines of the code; click one to go there.

A line above the editor says how the engine stands, with the button that helps: **Ready**; the code changed since it was built (**Build**); the code lacks a function the inspector names (**Write the missing functions**); or what to install to build it here. **Text** shows the declaration as Clarity text instead of the inspector. Building needs Rust with its WebAssembly target (`rustup target add wasm32-unknown-unknown`); Clarity says so when it is missing.

An engine built elsewhere, in another language, is a file only: put it in the `engines` folder and declare it. Its editor edits the declaration and says whether the file fits it.

## Declaring an engine

Put the file in the `engines` folder of the module folder, next to `resources`:

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
- The project tree lists the engines under Engines, marking one that does not fit its file. Clicking one opens its editor; see [Making an engine in WebAssembly](#making-an-engine-in-webassembly).

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

An engine that gives people something to read returns keys, and lists them with `says`:

```clarity
engine Board from "board.wasm"
    uses random
    says @yourMove, @youWin, @iWin, @aDraw
    says @thatSquareIsTaken
```

The engine returns `"youWin"`; the language files give the key its text, so the person reads it in their language, and a refusal by a key shows the same way. A text an engine gives that is no key shows as it is. An engine that needs the language itself, to sort or to format, uses `language`.

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

The easiest way to a WebAssembly engine is to let Clarity write it (see [Making an engine in WebAssembly](#making-an-engine-in-webassembly)): the code is Rust with the library `clarity-engine`, in a folder next to the file, `engines/pricing/` for `pricing.wasm`. The library writes the engine interface for you; a function takes its arguments as a tuple, in the declared order, and gives back anything serde writes:

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

An engine written in Clarity is code of the application on every stack, next to your functions: a struct of what it keeps, and the kept values of each window (or of the app) behind a lock. It needs no WebAssembly runtime.

Each stack hosts a WebAssembly file with a WebAssembly runtime written for it. You install nothing extra.

| Stack | Runs engines with | Comes from |
|---|---|---|
| Rust | wasmi | the `engines` feature of the crate `clarity-runtime`, turned on only when the project has an engine |
| Go | wazero | the package `engine` of the runtime module |
| C# | Wasmtime | the package `Clarity.Runtime.Engines`, referenced only when the project has an engine |
| C and C++ | wasm3 | copied into `clarity-runtime/` with the engine host, only when the project has an engine |

The generated program carries the engine inside it: embedded on Rust, Go and C#, as bytes in `src/engines.c` or `src/engines.cpp` on C and C++. Shared tests (`conformance/engines.json`) run the same calls through every host.

## An engine beside the screen

An engine can also draw a canvas, frame by frame, at the pace of the screen: a game, an animation, a simulation. It runs where the canvas is drawn, not on the backend, so a frame needs no trip to the backend.

```clarity
engine Screen from "screen.wasm"
    beside the screen
    on open
    limit 1 second

page Play: window
    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Screen
```

- **`beside the screen`** says where it runs. Code does not call it, so it has no `function` lines, and no `state` line: every canvas that `runs` it has an instance of its own, from when its window opens until it closes.
- **The frame loop.** At every frame the canvas calls the engine's `clarity_tick` with the milliseconds since the last frame (0 for the first), then `clarity_draw`, which answers the canvas's shapes in its units: the same shapes a draw handler draws (`clear`, `line`, `rect`, `circle`, `ring`, `text`). A canvas that runs an engine has no draw handler.
- **One frame at a time.** While the engine is busy with a frame, the screen's next frames are skipped; the next tick gives all the time since the last one, so nothing is lost.
- **Limits** hold per call: a frame that runs longer than the limit, or that fails, ends the engine. The canvas then says why in place of its drawing, in Clarity's words (`screenEngineFailed`), and the window works on.
- **`on open`** runs `clarity_open` once, before the first frame.
- **Where it runs.** On the web stacks in a Web Worker of its own, with the browser's WebAssembly, so an engine that loops never freezes the window; the engine's file travels in the model the screen reads. On Avalonia in Wasmtime, in the application's process, its frames worked off the screen's thread. The backend never loads it.

The file exports `clarity_tick` and `clarity_draw` besides the engine interface; the designer says so when it lacks them. Shared tests (`conformance/screen-engines.json`, with the engine `conformance/engines/screen`) run the same frames on both renderers.

## Not yet

These are designed but not built:

- The service `progress`, which would let a long call show on the screen how far it is.
- Pointer and keys for an engine beside the screen (`clarity_pointer`, `clarity_key`), what it tells the window (`tells`) and what a handler tells it (`hears`).
- Engines beside the screen written in Clarity (`on tick`, `on draw`).
