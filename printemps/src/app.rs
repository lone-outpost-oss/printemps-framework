
pub struct AppHandlerInitializing {
    pub wasm_engine: Option<wasmtime::Engine>,
    pub wasm_module: Option<wasmtime::Module>,
    pub wasm_linker: Option<wasmtime::Linker<()>>,
}

impl AppHandlerInitializing {
    pub fn new() -> Self {
        Self { 
            wasm_engine: None,
            wasm_module: None,
            wasm_linker: None,
        }
    }
}

pub struct AppHandler {
    pub wasm_engine: wasmtime::Engine,
    pub wasm_module: wasmtime::Module,
    pub wasm_linker: wasmtime::Linker<()>,
}

impl AppHandler {
    pub fn new(initialized: AppHandlerInitializing) -> Self {
        Self { 
            wasm_engine: initialized.wasm_engine.unwrap(),
            wasm_module: initialized.wasm_module.unwrap(),
            wasm_linker: initialized.wasm_linker.unwrap(),
        }
    }
}
