//! Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
//! which every stack hosts alike (conformance/engines.json). An engine
//! computes; it imports only the services of Clarity it declares (`uses
//! clock, random, log`), so it reaches no database, file or network.
//! Each call runs on a fresh instance of the module, so an engine keeps no
//! state from one call to the next, unless it is declared with `state per
//! app` (one instance serves every call) or `state per window` (each open
//! window has an instance of its own, dropped when the window closes). Every
//! call runs within the limits of its engine: a call that runs too long is
//! stopped, and an engine never gets more memory than its limit.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use wasmi::{
    Caller, CompilationMode, Config, ExternType, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc, TypedResumableCall, ValType,
    WasmParams, WasmResults,
};

use crate::{Error, Result, ui::Text};

/// What one call of an engine may take: how long it may run, and how much
/// memory the engine may grow to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub seconds: u32,
    pub megabytes: u32,
}

/// Whether an engine keeps an instance from call to call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Every call runs on a fresh instance.
    None,
    /// One instance serves every call of the application (`state per app`).
    App,
    /// Each open window has an instance of its own (`state per window`).
    Window,
}

/// An engine as its declaration gives it: its functions, the services it
/// uses, the limits of a call, whether it keeps state and whether it has
/// `on open`.
#[derive(Debug, Clone, Copy)]
pub struct Declaration<'a> {
    pub functions: &'a [&'a str],
    pub uses: &'a [&'a str],
    pub limits: Limits,
    pub state: State,
    /// Whether the engine's `clarity_open` runs on every new instance (`on open`).
    pub opens: bool,
}

/// The services an engine may use, each a function of module `clarity`
/// with its parameters and results.
const SERVICES: [(&str, &[ValType], &[ValType]); 4] = [
    ("clock", &[], &[ValType::I64]),
    ("random", &[], &[ValType::I64]),
    ("log", &[ValType::I32, ValType::I32], &[]),
    ("language", &[ValType::I32, ValType::I32], &[ValType::I32]),
];

/// What the services give: the system's clock, a fresh seed for every call
/// and the standard error, unless a test fixes the clock or the seed, or
/// keeps the lines of the log.
#[derive(Clone)]
pub struct Services {
    /// The time `clock` gives, in milliseconds since 1970; none for the system's clock.
    pub clock: Option<i64>,
    /// The seed `random` starts every call from; none for a fresh one each call.
    pub seed: Option<u64>,
    /// Where a line of the log goes, as `engine <Name>: <text>`.
    pub log: Arc<dyn Fn(&str) + Send + Sync>,
    /// The language `language` gives: the one the screen picked (crate::i18n); none for the texts as written.
    pub language: Arc<dyn Fn() -> Option<String> + Send + Sync>,
}

impl Default for Services {
    fn default() -> Self {
        Services { clock: None, seed: None, log: Arc::new(|line: &str| eprintln!("{line}")), language: Arc::new(crate::i18n::language) }
    }
}

/// What a call's store holds: its memory limit and the state of its services.
struct Data {
    limits: StoreLimits,
    name: String,
    clock: Option<i64>,
    /// The state of splitmix64, the random sequence every host gives alike.
    random: u64,
    log: Arc<dyn Fn(&str) + Send + Sync>,
    language: Arc<dyn Fn() -> Option<String> + Send + Sync>,
}

/// The next number of splitmix64.
fn splitmix(state: &mut u64) -> i64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as i64
}

/// How many instructions run between two looks at the clock. A debug
/// build of the interpreter takes stack as it runs and gives it back when a
/// slice ends, so its slices are short.
const SLICE: u64 = if cfg!(debug_assertions) { 1 << 14 } else { 1 << 20 };

/// An engine, its module checked and compiled once.
pub struct Engine {
    name: String,
    engine: wasmi::Engine,
    module: Module,
    uses: Vec<String>,
    limits: Limits,
    state: State,
    opens: bool,
    services: Services,
    /// The instances an engine with state keeps between calls, by window
    /// (one, at 0, for state per app).
    kept: Mutex<HashMap<u64, Live>>,
}

/// An instance with what a call needs of it.
struct Live {
    store: Store<Data>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    free: TypedFunc<(i32, i32), ()>,
    instance: wasmi::Instance,
}

/// Why a call ended without an answer.
enum Stop {
    /// It ran past its time limit.
    Late,
    Failed(String),
}

impl From<wasmi::Error> for Stop {
    fn from(error: wasmi::Error) -> Self {
        Stop::Failed(error.to_string())
    }
}

impl From<wasmi::errors::LinkerError> for Stop {
    fn from(error: wasmi::errors::LinkerError) -> Self {
        Stop::Failed(error.to_string())
    }
}

impl Engine {
    /// Loads `bytes` as engine `name` with the functions it is declared
    /// with, the services it uses and the limits of its calls; refuses a
    /// module that is no engine, imports anything but those services, or
    /// lacks one of the functions.
    pub fn load(name: &str, bytes: &[u8], declaration: &Declaration) -> Result<Engine> {
        let Declaration { functions, uses, limits, state, opens } = *declaration;
        let refused = |why: String| Error::Engine(format!("engine {name} {why}"));
        if !bytes.starts_with(b"\0asm") {
            return Err(refused("is no WebAssembly module".into()));
        }
        // Fuel counts instructions, so a call runs in slices with a look at the clock between them.
        // Every function is compiled when the engine loads, so no call pays for compiling with its fuel.
        let mut config = Config::default();
        config.consume_fuel(true).compilation_mode(CompilationMode::Eager);
        let engine = wasmi::Engine::new(&config);
        let module = Module::new(&engine, bytes).map_err(|e| refused(format!("is no WebAssembly module: {e}")))?;
        for import in module.imports() {
            let used = import.module() == "clarity" && uses.contains(&import.name());
            if !used {
                return Err(refused(format!("imports {}.{}; an engine imports only the services it uses", import.module(), import.name())));
            }
            let fits = SERVICES.iter().any(|(service, params, results)| {
                *service == import.name() && matches!(import.ty(), ExternType::Func(ty) if ty.params() == *params && ty.results() == *results)
            });
            if !fits {
                return Err(refused(format!("imports clarity.{} with another type than Clarity gives it", import.name())));
            }
        }
        let exported = |wanted: &str| module.exports().any(|e| e.name() == wanted && matches!(e.ty(), ExternType::Func(_)));
        for needed in ["clarity_alloc", "clarity_free"] {
            if !exported(needed) {
                return Err(refused(format!("lacks {needed} of the engine interface")));
            }
        }
        if !module.exports().any(|e| e.name() == "memory" && matches!(e.ty(), ExternType::Memory(_))) {
            return Err(refused("lacks the memory of the engine interface".into()));
        }
        if let Some(missing) = functions.iter().find(|f| !exported(f)) {
            return Err(refused(format!("has no function {missing}")));
        }
        if opens && !exported("clarity_open") {
            return Err(refused("lacks clarity_open, which its on open needs".into()));
        }
        let uses = uses.iter().map(|u| u.to_string()).collect();
        Ok(Engine { name: name.to_string(), engine, module, uses, limits, state, opens, services: Services::default(), kept: Mutex::new(HashMap::new()) })
    }

    /// The engine with `services` in place of the system's.
    pub fn with_services(mut self, services: Services) -> Engine {
        self.services = services;
        self
    }

    /// Calls `function` with `args`: its value, its refusal in its own
    /// words, or what went wrong running it.
    pub fn call(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        self.call_in(0, function, args)
    }

    /// Calls `function` in open window `window`: for an engine with state
    /// per window, on that window's own instance.
    pub fn call_in(&self, window: u64, function: &str, args: Vec<Value>) -> Result<Value> {
        // The interpreter runs on a thread of its own with room to spare, so a
        // deep engine never overflows the caller's stack (a debug build needs it).
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(16 << 20)
                .spawn_scoped(scope, || self.run(window, function, args))
                .map_err(|e| Error::Engine(format!("engine {} could not start {function}: {e}", self.name)))?
                .join()
                .unwrap_or_else(|_| Err(Error::Engine(format!("engine {} failed in {function}", self.name))))
        })
    }

    /// Drops the instance window `window` had, when it closes.
    pub fn close_window(&self, window: u64) {
        if self.state == State::Window {
            self.kept.lock().unwrap_or_else(|e| e.into_inner()).remove(&window);
        }
    }

    fn run(&self, window: u64, function: &str, args: Vec<Value>) -> Result<Value> {
        let failed = |why: String| Error::Engine(format!("engine {} failed in {function}: {why}", self.name));
        let seconds = self.limits.seconds;
        let until = Instant::now() + Duration::from_secs(u64::from(seconds));
        let stopped = |stop| match stop {
            Stop::Late => failed(format!("it ran longer than {seconds} second{}", if seconds == 1 { "" } else { "s" })),
            Stop::Failed(why) => failed(why),
        };
        // An engine with state keeps its instance, unless a call fails or runs
        // past its limit: then its memory may be half changed, so the next
        // call starts a new one.
        let key = match self.state {
            State::None => None,
            State::App => Some(0),
            State::Window => Some(window),
        };
        let mut kept = key.map(|_| self.kept.lock().unwrap_or_else(|e| e.into_inner()));
        let mut live = match key.and_then(|k| kept.as_mut().and_then(|kept| kept.remove(&k))) {
            Some(live) => live,
            None => self.instance(until).map_err(stopped)?,
        };
        let output = invoke(&mut live, function, args, until, self.services.seed).map_err(stopped)?;
        // The engine answered, even with a refusal, so its memory is whole.
        if let (Some(key), Some(kept)) = (key, kept.as_mut()) {
            kept.insert(key, live);
        }
        self.answer(function, &output)
    }

    /// The value in an engine's answer, or its refusal in its own words.
    fn answer(&self, function: &str, output: &[u8]) -> Result<Value> {
        let failed = |why: String| Error::Engine(format!("engine {} failed in {function}: {why}", self.name));
        let answer: Value = serde_json::from_slice(output).map_err(|e| failed(format!("its answer is no JSON: {e}")))?;
        match (answer.get("value"), answer.get("error")) {
            // An engine refuses by the key of its text (its `says`), or in its own words.
            (_, Some(error)) => Err(Error::Refused(Text::key(error.as_str().unwrap_or_default()))),
            (Some(value), None) => Ok(value.clone()),
            (None, None) => Err(failed("its answer has no value".into())),
        }
    }

    /// A new instance of the module with its services, opened when the engine has `on open`.
    fn instance(&self, until: Instant) -> std::result::Result<Live, Stop> {
        let memory_limit = self.limits.megabytes as usize * 1024 * 1024;
        let data = Data {
            limits: StoreLimitsBuilder::new().memory_size(memory_limit).build(),
            name: self.name.clone(),
            clock: self.services.clock,
            random: 0,
            log: self.services.log.clone(),
            language: self.services.language.clone(),
        };
        let mut store = Store::new(&self.engine, data);
        store.limiter(|data: &mut Data| &mut data.limits);
        store.set_fuel(SLICE)?;
        let instance = self.linker()?.instantiate_and_start(&mut store, &self.module)?;
        let memory: Memory = instance.get_memory(&store, "memory").ok_or_else(|| Stop::Failed("no memory".into()))?;
        let alloc: TypedFunc<i32, i32> = instance.get_typed_func(&store, "clarity_alloc")?;
        let free: TypedFunc<(i32, i32), ()> = instance.get_typed_func(&store, "clarity_free")?;
        let mut live = Live { store, memory, alloc, free, instance };
        if self.opens {
            let output = invoke(&mut live, "clarity_open", Vec::new(), until, self.services.seed)?;
            match self.answer("on open", &output) {
                Ok(_) => {}
                Err(Error::Refused(why)) => return Err(Stop::Failed(format!("its on open refused: {}", why.show()))),
                Err(_) => return Err(Stop::Failed("its on open gave no answer".into())),
            }
        }
        Ok(live)
    }
}

/// Writes the arguments into the instance, runs `function` and reads its
/// answer. The random sequence starts afresh for every call.
fn invoke(live: &mut Live, function: &str, args: Vec<Value>, until: Instant, seed: Option<u64>) -> std::result::Result<Vec<u8>, Stop> {
    live.store.data_mut().random = seed.unwrap_or_else(|| std::collections::hash_map::RandomState::new().build_hasher().finish());
    let run: TypedFunc<(i32, i32), i64> = live.instance.get_typed_func(&live.store, function)?;
    let input = serde_json::to_vec(&Value::Array(args)).map_err(|e| Stop::Failed(e.to_string()))?;
    let len = i32::try_from(input.len()).map_err(|_| Stop::Failed("the arguments are too large".into()))?;
    let (alloc, free, memory) = (live.alloc, live.free, live.memory);
    let at = timed(&mut live.store, &alloc, len, until)?;
    memory.write(&mut live.store, at as u32 as usize, &input).map_err(|e| Stop::Failed(e.to_string()))?;
    let packed = timed(&mut live.store, &run, (at, len), until)?;
    let (place, size) = ((packed >> 32) as u32, packed as u32);
    let mut output = vec![0; size as usize];
    memory.read(&live.store, place as usize, &mut output).map_err(|e| Stop::Failed(e.to_string()))?;
    timed(&mut live.store, &free, (place as i32, size as i32), until)?;
    Ok(output)
}

impl Engine {
    /// The services the engine uses, as module `clarity`.
    fn linker(&self) -> std::result::Result<Linker<Data>, Stop> {
        let mut linker = Linker::<Data>::new(&self.engine);
        let uses = |service: &str| self.uses.iter().any(|u| u == service);
        if uses("clock") {
            linker.func_wrap("clarity", "clock", |caller: Caller<'_, Data>| -> i64 {
                caller.data().clock.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64))
            })?;
        }
        if uses("random") {
            linker.func_wrap("clarity", "random", |mut caller: Caller<'_, Data>| -> i64 { splitmix(&mut caller.data_mut().random) })?;
        }
        if uses("log") {
            linker.func_wrap("clarity", "log", |caller: Caller<'_, Data>, at: i32, len: i32| -> std::result::Result<(), wasmi::Error> {
                let memory = caller.get_export("memory").and_then(|e| e.into_memory()).ok_or_else(|| wasmi::Error::new("no memory"))?;
                let mut text = vec![0; len.max(0) as usize];
                memory.read(&caller, at as u32 as usize, &mut text).map_err(|e| wasmi::Error::new(e.to_string()))?;
                let data = caller.data();
                (data.log)(&format!("engine {}: {}", data.name, String::from_utf8_lossy(&text)));
                Ok(())
            })?;
        }
        if uses("language") {
            // The language as it is at the call, written into the engine's room.
            linker.func_wrap("clarity", "language", |mut caller: Caller<'_, Data>, at: i32, room: i32| -> std::result::Result<i32, wasmi::Error> {
                let memory = caller.get_export("memory").and_then(|e| e.into_memory()).ok_or_else(|| wasmi::Error::new("no memory"))?;
                let language = (caller.data().language)().unwrap_or_default();
                let bytes = language.as_bytes();
                let written = bytes.len().min(room.max(0) as usize);
                memory.write(&mut caller, at as u32 as usize, &bytes[..written]).map_err(|e| wasmi::Error::new(e.to_string()))?;
                Ok(bytes.len() as i32)
            })?;
        }
        Ok(linker)
    }
}

/// Runs `func` a slice of fuel at a time until it ends, or stops it once
/// `until` has passed.
fn timed<P: WasmParams, R: WasmResults>(store: &mut Store<Data>, func: &TypedFunc<P, R>, params: P, until: Instant) -> std::result::Result<R, Stop> {
    let mut call = func.call_resumable(&mut *store, params)?;
    loop {
        match call {
            TypedResumableCall::Finished(results) => return Ok(results),
            TypedResumableCall::OutOfFuel(paused) => {
                if Instant::now() >= until {
                    return Err(Stop::Late);
                }
                // A slice holds at least the next block of instructions.
                store.set_fuel(SLICE.max(paused.required_fuel()))?;
                call = paused.resume(&mut *store)?;
            }
            TypedResumableCall::HostTrap(trap) => return Err(Stop::Failed(trap.host_error().to_string())),
        }
    }
}

/// A value of code as it goes to an engine.
pub fn to_value<T: serde::Serialize + ?Sized>(value: &T) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| Error::Engine(format!("a value for an engine: {e}")))
}

/// Bytes as they go to an engine: base64 text.
pub fn bytes_value(bytes: &[u8]) -> Value {
    use base64::Engine as _;
    Value::String(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// The bytes an engine gave, from base64 text.
pub fn bytes_from(engine: &str, function: &str, value: Value) -> Result<Vec<u8>> {
    use base64::Engine as _;
    let wrong = || Error::Engine(format!("engine {engine} gave {function} a value of another type: no bytes"));
    let text = value.as_str().ok_or_else(wrong)?;
    base64::engine::general_purpose::STANDARD.decode(text).map_err(|_| wrong())
}

/// What an engine gave, as a value of code of its declared type.
pub fn from_value<T: serde::de::DeserializeOwned>(engine: &str, function: &str, value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|e| Error::Engine(format!("engine {engine} gave {function} a value of another type: {e}")))
}
