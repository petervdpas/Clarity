//! An engine as the interpreter runs it: the support module built with the
//! interpreter, the engine's IR as data, and per function an export that
//! calls `rt_run` with the IR's place, the function's number, the call's
//! arguments and the function-table entry of `random`.

use clarity_engine_ir as ir;
use wasm_encoder::{Function, Instruction as I, ValType};

use crate::emit::SERVICES;
use crate::merge::{Code, Support};

pub fn engine(support: &Support, engine: &ir::Engine) -> Result<Code, String> {
    let mut code = Code::default();
    for name in &engine.services {
        let (_, params, results) = SERVICES.iter().find(|(n, ..)| n == name).expect("a service of Clarity");
        let ty = code.ty(params.to_vec(), results.to_vec());
        code.imports.push((name.clone(), ty));
    }
    // The host's random, reached through the function table.
    let random = match engine.services.iter().position(|s| s == "random") {
        Some(import) => {
            code.table.push(import as u32);
            support.table_entry(0) as i32
        }
        None => -1,
    };
    code.texts = serde_json::to_vec(engine).map_err(|e| e.to_string())?;
    let (at, len) = (support.texts_at() as i32, code.texts.len() as i32);
    let run = support.rt("rt_run");
    let ty = code.ty(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
    for (index, function) in engine.functions.iter().enumerate() {
        let mut export = Function::new(Vec::<(u32, ValType)>::new());
        for instruction in [I::I32Const(at), I::I32Const(len), I::I32Const(index as i32), I::LocalGet(0), I::LocalGet(1), I::I32Const(random), I::Call(run), I::End] {
            export.instruction(&instruction);
        }
        code.functions.push((ty, export));
        code.exports.push((function.name.clone(), index as u32));
    }
    Ok(code)
}
