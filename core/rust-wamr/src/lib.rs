pub mod runtime;
pub mod loader;
pub mod interpreter;
pub mod memory;
pub mod aot_runtime;
pub mod jit;
pub mod c_api;

pub use runtime::WasmError;
pub use runtime::TrapCode;
pub use runtime::ValType;
pub use runtime::RefType;
pub use runtime::NumType;
pub use runtime::FunctionType;
pub use runtime::GlobalType;
pub use runtime::TableType;
pub use runtime::MemoryType;
pub use runtime::Table;
pub use runtime::Memory;
pub use runtime::Global;
pub use runtime::ExportType;
pub use runtime::ImportType;
pub use runtime::Module;
pub use runtime::Instance;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
