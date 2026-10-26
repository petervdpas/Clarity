//! The support module, and the engine's code added to it.
//!
//! The support module imports nothing. The engine imports the services it
//! uses, which come first among the functions, so the support module's move
//! up by as many and the engine's come after them. Its texts go in a data segment where
//! the support module's memory ended, which grows by as many pages: the
//! allocator takes memory only by growing it, so nothing else is there.

use std::collections::HashMap;

use wasm_encoder::reencode::Reencode;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ElementSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, GlobalSection, ImportSection,
    MemorySection, MemoryType, Module, TableSection, TypeSection, ValType,
};
use wasmparser::{ExternalKind, Parser, Payload};

/// The bytes of a wasm page.
const PAGE: u64 = 65536;

/// What the compiler needs to know of the support module.
pub struct Support {
    types: u32,
    functions: u32,
    /// The `rt_*` functions by name, and their numbers.
    rt: HashMap<String, u32>,
    pages: u64,
    /// How many services the engine imports, before every function.
    imports: u32,
}

/// The support module's code with its functions moved up past the imports.
struct Shift(u32);

impl Reencode for Shift {
    type Error = std::convert::Infallible;

    fn function_index(&mut self, func: u32) -> u32 {
        func + self.0
    }
}

impl Support {
    pub fn read(bytes: &[u8]) -> Result<Support, String> {
        let mut support = Support { types: 0, functions: 0, rt: HashMap::new(), pages: 0, imports: 0 };
        for payload in Parser::new(0).parse_all(bytes) {
            match payload.map_err(|e| e.to_string())? {
                Payload::TypeSection(r) => support.types = r.count(),
                Payload::ImportSection(r) if r.count() > 0 => return Err("it imports, so its functions would move".into()),
                Payload::FunctionSection(r) => support.functions = r.count(),
                Payload::MemorySection(r) => {
                    for memory in r {
                        support.pages = memory.map_err(|e| e.to_string())?.initial;
                    }
                }
                Payload::ExportSection(r) => {
                    for export in r {
                        let export = export.map_err(|e| e.to_string())?;
                        if export.kind == ExternalKind::Func && export.name.starts_with("rt_") {
                            support.rt.insert(export.name.to_string(), export.index);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(support)
    }

    /// The engine imports `count` services.
    pub fn importing(&mut self, count: u32) {
        self.imports = count;
    }

    /// The number of a support function the compiled code calls.
    pub fn rt(&self, name: &str) -> u32 {
        self.imports + *self.rt.get(name).unwrap_or_else(|| panic!("the support module has no {name}"))
    }

    /// The number the engine's `index`th function gets.
    pub fn function(&self, index: u32) -> u32 {
        self.imports + self.functions + index
    }

    /// Where the engine's texts start in memory.
    pub fn texts_at(&self) -> u32 {
        (self.pages * PAGE) as u32
    }
}

/// The engine's code: its function types, functions, exports and texts.
#[derive(Default)]
pub struct Code {
    pub types: Vec<(Vec<ValType>, Vec<ValType>)>,
    /// Each function with the number of its type in `types`.
    pub functions: Vec<(u32, Function)>,
    /// The functions the host calls, by name and number among `functions`.
    pub exports: Vec<(String, u32)>,
    pub texts: Vec<u8>,
    /// The services it imports from `clarity`, by name, with the number of their type in `types`.
    pub imports: Vec<(String, u32)>,
}

impl Code {
    /// The number of a function type, added when it is new.
    pub fn ty(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let ty = (params, results);
        match self.types.iter().position(|t| *t == ty) {
            Some(at) => at as u32,
            None => {
                self.types.push(ty);
                self.types.len() as u32 - 1
            }
        }
    }
}

/// The interface the engine keeps from the support module's exports.
const INTERFACE: [&str; 3] = ["memory", "clarity_alloc", "clarity_free"];

pub fn write(bytes: &[u8], support: &Support, code: Code) -> Result<Vec<u8>, String> {
    let mut re = Shift(support.imports);
    let mut module = Module::new();
    let pages_for_texts = (code.texts.len() as u64).div_ceil(PAGE);
    let mut functions_left = Some(code.functions);
    let mut code_section = CodeSection::new();
    let mut data_written = false;
    let err = |e: wasm_encoder::reencode::Error| format!("{e:?}");
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| e.to_string())? {
            Payload::TypeSection(r) => {
                let mut types = TypeSection::new();
                re.parse_type_section(&mut types, r).map_err(err)?;
                for (params, results) in &code.types {
                    types.ty().function(params.iter().copied(), results.iter().copied());
                }
                module.section(&types);
                if !code.imports.is_empty() {
                    let mut imports = ImportSection::new();
                    for (name, ty) in &code.imports {
                        imports.import("clarity", name, EntityType::Function(support.types + ty));
                    }
                    module.section(&imports);
                }
            }
            Payload::FunctionSection(r) => {
                let mut functions = FunctionSection::new();
                re.parse_function_section(&mut functions, r).map_err(err)?;
                for (ty, _) in functions_left.as_ref().into_iter().flatten() {
                    functions.function(support.types + ty);
                }
                module.section(&functions);
            }
            Payload::TableSection(r) => {
                let mut tables = TableSection::new();
                re.parse_table_section(&mut tables, r).map_err(err)?;
                module.section(&tables);
            }
            Payload::MemorySection(r) => {
                let mut memories = MemorySection::new();
                for memory in r {
                    let memory = memory.map_err(|e| e.to_string())?;
                    let minimum = memory.initial + pages_for_texts;
                    memories.memory(MemoryType {
                        minimum,
                        maximum: memory.maximum.map(|m| m.max(minimum)),
                        memory64: memory.memory64,
                        shared: memory.shared,
                        page_size_log2: memory.page_size_log2,
                    });
                }
                module.section(&memories);
            }
            Payload::GlobalSection(r) => {
                let mut globals = GlobalSection::new();
                re.parse_global_section(&mut globals, r).map_err(err)?;
                module.section(&globals);
            }
            Payload::ExportSection(r) => {
                let mut exports = ExportSection::new();
                for export in r {
                    let export = export.map_err(|e| e.to_string())?;
                    if INTERFACE.contains(&export.name) {
                        re.parse_export(&mut exports, export);
                    }
                }
                for (name, index) in &code.exports {
                    exports.export(name, ExportKind::Func, support.function(*index));
                }
                module.section(&exports);
            }
            Payload::ElementSection(r) => {
                let mut elements = ElementSection::new();
                re.parse_element_section(&mut elements, r).map_err(err)?;
                module.section(&elements);
            }
            Payload::CodeSectionEntry(body) => {
                re.parse_function_body(&mut code_section, body).map_err(err)?;
                // The last of the support module's functions: the engine's follow.
                if code_section.len() == support.functions {
                    for (_, function) in functions_left.take().into_iter().flatten() {
                        code_section.function(&function);
                    }
                    module.section(&code_section);
                }
            }
            Payload::DataSection(r) => {
                let mut data = DataSection::new();
                re.parse_data_section(&mut data, r).map_err(err)?;
                if !code.texts.is_empty() {
                    data.active(0, &ConstExpr::i32_const(support.texts_at() as i32), code.texts.iter().copied());
                }
                module.section(&data);
                data_written = true;
            }
            Payload::DataCountSection { .. } => return Err("the support module counts its data, which the texts would change".into()),
            _ => {}
        }
    }
    if !data_written && !code.texts.is_empty() {
        let mut data = DataSection::new();
        data.active(0, &ConstExpr::i32_const(support.texts_at() as i32), code.texts.iter().copied());
        module.section(&data);
    }
    Ok(module.finish())
}
