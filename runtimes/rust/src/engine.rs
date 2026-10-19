//! Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
//! which every stack hosts alike (conformance/engines.json). An engine
//! computes; it imports nothing, so it reaches no database, file or network.
//! Each call runs on a fresh instance of the module, so an engine keeps no
//! state from one call to the next.

use serde_json::Value;
use wasmi::{ExternType, Linker, Memory, Module, Store, TypedFunc};

use crate::{Error, Result, ui::Text};

/// An engine, its module checked and compiled once.
pub struct Engine {
    name: String,
    engine: wasmi::Engine,
    module: Module,
}

impl Engine {
    /// Loads `bytes` as engine `name` with the functions it is declared
    /// with; refuses a module that is no engine, imports anything, or lacks
    /// one of them.
    pub fn load(name: &str, bytes: &[u8], functions: &[&str]) -> Result<Engine> {
        let refused = |why: String| Error::Engine(format!("engine {name} {why}"));
        if !bytes.starts_with(b"\0asm") {
            return Err(refused("is no WebAssembly module".into()));
        }
        let engine = wasmi::Engine::default();
        let module = Module::new(&engine, bytes).map_err(|e| refused(format!("is no WebAssembly module: {e}")))?;
        if let Some(import) = module.imports().next() {
            return Err(refused(format!("imports {}.{}; an engine imports nothing", import.module(), import.name())));
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
        Ok(Engine { name: name.to_string(), engine, module })
    }

    /// Calls `function` with `args`: its value, its refusal in its own
    /// words, or what went wrong running it.
    pub fn call(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        // The interpreter runs on a thread of its own with room to spare, so a
        // deep engine never overflows the caller's stack (a debug build needs it).
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(16 << 20)
                .spawn_scoped(scope, || self.run(function, args))
                .map_err(|e| Error::Engine(format!("engine {} could not start {function}: {e}", self.name)))?
                .join()
                .unwrap_or_else(|_| Err(Error::Engine(format!("engine {} failed in {function}", self.name))))
        })
    }

    fn run(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        let failed = |why: String| Error::Engine(format!("engine {} failed in {function}: {why}", self.name));
        let mut store = Store::new(&self.engine, ());
        let instance = Linker::<()>::new(&self.engine).instantiate_and_start(&mut store, &self.module).map_err(|e| failed(e.to_string()))?;
        let memory: Memory = instance.get_memory(&store, "memory").ok_or_else(|| failed("no memory".into()))?;
        let alloc: TypedFunc<i32, i32> = instance.get_typed_func(&store, "clarity_alloc").map_err(|e| failed(e.to_string()))?;
        let free: TypedFunc<(i32, i32), ()> = instance.get_typed_func(&store, "clarity_free").map_err(|e| failed(e.to_string()))?;
        let run: TypedFunc<(i32, i32), i64> = instance.get_typed_func(&store, function).map_err(|e| failed(e.to_string()))?;

        let input = serde_json::to_vec(&Value::Array(args)).map_err(|e| failed(e.to_string()))?;
        let len = i32::try_from(input.len()).map_err(|_| failed("the arguments are too large".into()))?;
        let at = alloc.call(&mut store, len).map_err(|e| failed(e.to_string()))?;
        memory.write(&mut store, at as u32 as usize, &input).map_err(|e| failed(e.to_string()))?;
        let packed = run.call(&mut store, (at, len)).map_err(|e| failed(e.to_string()))?;
        let (place, size) = ((packed >> 32) as u32, packed as u32);
        let mut output = vec![0; size as usize];
        memory.read(&store, place as usize, &mut output).map_err(|e| failed(e.to_string()))?;
        free.call(&mut store, (place as i32, size as i32)).map_err(|e| failed(e.to_string()))?;

        let answer: Value = serde_json::from_slice(&output).map_err(|e| failed(format!("its answer is no JSON: {e}")))?;
        match (answer.get("value"), answer.get("error")) {
            (_, Some(error)) => Err(Error::Refused(Text::plain(error.as_str().unwrap_or_default()))),
            (Some(value), None) => Ok(value.clone()),
            (None, None) => Err(failed("its answer has no value".into())),
        }
    }
}

/// A value of code as it goes to an engine.
pub fn to_value<T: serde::Serialize + ?Sized>(value: &T) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| Error::Engine(format!("a value for an engine: {e}")))
}

/// What an engine gave, as a value of code of its declared type.
pub fn from_value<T: serde::de::DeserializeOwned>(engine: &str, function: &str, value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|e| Error::Engine(format!("engine {engine} gave {function} a value of another type: {e}")))
}
