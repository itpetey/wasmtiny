//! WasmApplication - High-level API for Wasm module execution.
//!
//! This module provides the main entry point for interacting with the WebAssembly
//! runtime. [`WasmApplication`] manages module loading, instantiation, host function
//! registration, and function invocation.
//!
//! # Example
//!
//! ```ignore
//! use wasmtiny::{WasmApplication, WasmValue};
//!
//! let mut app = WasmApplication::new();
//! let idx = app.load_module_from_file("hello.wasm")?;
//! app.instantiate(idx)?;
//! let result = app.call_function(idx, "main", &[])?;
//! ```

use crate::aot_runtime::runtime::{AotExport, AotRuntime};
use crate::runtime::{
    Extern, FunctionType, Global, GuestFuncBinding, HostFunc, Import, InstanceLimits,
    InstanceStats, Memory, Result, SharedMemoryMappingId, SharedRegionId, SharedTable, Table,
    WasmError, WasmValue,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "llvm-jit")]
use crate::jit::{LlvmJit, set_execution_context};
#[cfg(feature = "llvm-jit")]
use std::collections::HashMap;

/// Execution mode for WebAssembly modules.
///
/// Specifies whether to use the interpreter or LLVM JIT compiler.
pub enum ExecutionMode {
    /// Execute using the interpreter.
    Interpreter,
    /// Execute using LLVM JIT compilation.
    #[cfg(feature = "llvm-jit")]
    /// Execute using the LLVM JIT backend.
    LlvmJit,
}

/// A WebAssembly application instance.
///
/// This is the main entry point for interacting with the WebAssembly runtime.
/// It manages module loading, instantiation, host function registration, and
/// function invocation.
///
/// # Example
///
/// ```ignore
/// use wasmtiny::{WasmApplication, WasmValue};
///
/// let mut app = WasmApplication::new();
/// let idx = app.load_module_from_file("module.wasm")?;
/// app.instantiate(idx)?;
/// let result = app.call_function(idx, "add", &[WasmValue::I32(1), WasmValue::I32(2)])?;
/// ```
pub struct WasmApplication {
    runtime: AotRuntime,
    #[cfg(feature = "llvm-jit")]
    llvm_jits: HashMap<u32, LlvmJit>,
    #[cfg(feature = "llvm-jit")]
    execution_mode: ExecutionMode,
}

impl WasmApplication {
    #[cfg(feature = "llvm-jit")]
    fn validate_llvm_compatibility(module: &crate::runtime::Module) -> Result<()> {
        if module
            .imports
            .iter()
            .any(|import| matches!(import.kind, crate::runtime::ImportKind::Memory(_)))
        {
            return Err(WasmError::Runtime(
                "LLVM JIT does not support imported memories".to_string(),
            ));
        }

        for (type_idx, func_type) in module.types.iter().enumerate() {
            if func_type.results.len() > 1 {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support multi-value function type {}",
                    type_idx
                )));
            }
            if func_type
                .params
                .iter()
                .chain(func_type.results.iter())
                .any(|value_type| value_type.is_reference())
            {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support reference-typed function type {}",
                    type_idx
                )));
            }
        }

        for (func_idx, func) in module.funcs.iter().enumerate() {
            if func.locals.iter().any(|local| local.type_.is_reference()) {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support reference-typed locals in function {}",
                    func_idx
                )));
            }
        }

        Ok(())
    }

    /// Creates a new `WasmApplication`.
    pub fn new() -> Self {
        Self {
            runtime: AotRuntime::new(),
            #[cfg(feature = "llvm-jit")]
            llvm_jits: HashMap::new(),
            #[cfg(feature = "llvm-jit")]
            execution_mode: ExecutionMode::Interpreter,
        }
    }

    #[cfg(feature = "llvm-jit")]
    /// Sets execution mode.
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.execution_mode = mode;
    }

    #[cfg(feature = "llvm-jit")]
    /// Returns the current execution mode.
    pub fn execution_mode(&self) -> &ExecutionMode {
        &self.execution_mode
    }

    #[cfg(feature = "llvm-jit")]
    /// Compiles the selected module with the LLVM JIT backend.
    pub fn compile_with_llvm(&mut self, module_idx: u32) -> Result<()> {
        let aot_module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        Self::validate_llvm_compatibility(aot_module.module())?;

        let module_name = format!("module_{}", module_idx);
        let mut llvm_jit = LlvmJit::new(&module_name)?;

        match llvm_jit.compile_module(aot_module.module()) {
            Ok(_) => {
                self.llvm_jits.insert(module_idx, llvm_jit);
                self.execution_mode = ExecutionMode::LlvmJit;
                Ok(())
            }
            Err(e) => {
                self.llvm_jits.remove(&module_idx);
                self.execution_mode = ExecutionMode::Interpreter;
                Err(e)
            }
        }
    }

    #[cfg(feature = "llvm-jit")]
    /// Attempts LLVM compilation and falls back if it is unavailable.
    pub fn compile_with_llvm_fallback(&mut self, module_idx: u32) {
        let _ = self.compile_with_llvm(module_idx);
    }

    /// Loads module from file.
    pub fn load_module_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<u32> {
        let data = fs::read(path)?;
        self.load_module_from_memory(&data)
    }

    /// Loads module from memory.
    pub fn load_module_from_memory(&mut self, data: &[u8]) -> Result<u32> {
        self.runtime.load_module(data)
    }

    /// Instantiates the module and resolves its imports.
    pub fn instantiate(&mut self, module_idx: u32) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.instantiate()
    }

    /// Returns runtime statistics for the selected instance.
    pub fn instance_stats(&self, module_idx: u32) -> Result<InstanceStats> {
        self.runtime.instance_stats(module_idx)
    }

    /// Returns the configured limits for the selected instance.
    pub fn instance_limits(&self, module_idx: u32) -> Result<InstanceLimits> {
        self.runtime.instance_limits(module_idx)
    }

    /// Sets instance limits.
    pub fn set_instance_limits(&mut self, module_idx: u32, limits: InstanceLimits) -> Result<()> {
        self.runtime.set_instance_limits(module_idx, limits)
    }

    /// Registers host function.
    pub fn register_host_function(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        func: Box<dyn HostFunc>,
        func_type: FunctionType,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_host_import(import_module, name, func, func_type)
    }

    /// Registers memory import.
    pub fn register_memory_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        memory: crate::runtime::Memory,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_memory_import(import_module, name, memory)
    }

    /// Registers table import.
    pub fn register_table_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        table: crate::runtime::Table,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_table_import(import_module, name, table)
    }

    /// Registers global import.
    pub fn register_global_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        global: crate::runtime::Global,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_global_import(import_module, name, global)
    }

    /// Returns the declared imports for a module.
    pub fn imports(&self, module_idx: u32) -> Result<Vec<Import>> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        Ok(module.imports().to_vec())
    }

    /// Returns an exported memory by name.
    pub fn export_memory(&self, module_idx: u32, name: &str) -> Result<Memory> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Memory(0)) => module.get_memory().ok_or_else(|| {
                WasmError::Runtime(format!("memory export {} is unavailable", name))
            }),
            Some(AotExport::Memory(idx)) => Err(WasmError::Runtime(format!(
                "memory export {} uses unsupported memory index {}",
                name, idx
            ))),
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a memory",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "memory export {} not found",
                name
            ))),
        }
    }

    /// Returns an exported table by name.
    pub fn export_table(&self, module_idx: u32, name: &str) -> Result<Table> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Table(idx)) => module
                .get_table(*idx)
                .ok_or_else(|| WasmError::Runtime(format!("table export {} is unavailable", name))),
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a table",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "table export {} not found",
                name
            ))),
        }
    }

    /// Returns the index of an exported table by name.
    pub fn export_table_index(&self, module_idx: u32, name: &str) -> Result<u32> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Table(idx)) => Ok(*idx),
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a table",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "table export {} not found",
                name
            ))),
        }
    }

    /// Returns a table by index.
    pub fn table(&self, module_idx: u32, table_idx: u32) -> Result<Table> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module
            .get_table(table_idx)
            .ok_or_else(|| WasmError::Runtime(format!("table {} not found", table_idx)))
    }

    /// Returns a shared table binding by index.
    pub fn table_binding(&self, module_idx: u32, table_idx: u32) -> Result<SharedTable> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module
            .table_binding(table_idx)
            .ok_or_else(|| WasmError::Runtime(format!("table {} not found", table_idx)))
    }

    /// Replaces a table by index.
    pub fn set_table(&mut self, module_idx: u32, table_idx: u32, table: Table) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.set_table(table_idx, table)
    }

    /// Returns an exported global by name.
    pub fn export_global(&self, module_idx: u32, name: &str) -> Result<Global> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Global(idx)) => module.get_global(*idx).ok_or_else(|| {
                WasmError::Runtime(format!("global export {} is unavailable", name))
            }),
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a global",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "global export {} not found",
                name
            ))),
        }
    }

    /// Returns the function type of an exported function by name.
    pub fn func_type(&self, module_idx: u32, name: &str) -> Result<FunctionType> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Function(idx)) => {
                module.module().func_type(*idx).cloned().ok_or_else(|| {
                    WasmError::Runtime(format!("function export {} is unavailable", name))
                })
            }
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a function",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "function export {} not found",
                name
            ))),
        }
    }

    /// Returns an exported function binding suitable for another module import.
    pub fn function_binding(&self, module_idx: u32, name: &str) -> Result<GuestFuncBinding> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Function(idx)) => Ok(GuestFuncBinding {
                module: Arc::new(module.module().clone()),
                imports: module
                    .imports()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, import)| {
                        module
                            .import_binding(i)
                            .cloned()
                            .map(|extern_| (import.module.clone(), import.name.clone(), extern_))
                    })
                    .collect(),
                func_idx: *idx,
                func_type: module.module().func_type(*idx).cloned().ok_or_else(|| {
                    WasmError::Runtime(format!("function export {} is unavailable", name))
                })?,
            }),
            Some(_) => Err(WasmError::Runtime(format!(
                "export {} is not a function",
                name
            ))),
            None => Err(WasmError::Runtime(format!(
                "function export {} not found",
                name
            ))),
        }
    }

    /// Returns the function type of an exported tag by name.
    pub fn tag_type(&self, module_idx: u32, name: &str) -> Result<FunctionType> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        match module.get_export(name) {
            Some(AotExport::Tag(idx)) => {
                module.module().tag_type(*idx).cloned().ok_or_else(|| {
                    WasmError::Runtime(format!("tag export {} is unavailable", name))
                })
            }
            Some(_) => Err(WasmError::Runtime(format!("export {} is not a tag", name))),
            None => Err(WasmError::Runtime(format!("tag export {} not found", name))),
        }
    }

    /// Registers a tag import by name.
    pub fn register_tag_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        function_type: FunctionType,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_import(import_module, name, Extern::Tag(function_type))
    }

    /// Registers a guest function import by binding.
    pub fn register_function_import_binding(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        binding: GuestFuncBinding,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_import(import_module, name, Extern::Func(binding))
    }

    /// Registers a shared table import binding.
    pub fn register_table_import_binding(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        table: SharedTable,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_import(import_module, name, Extern::Table(table))
    }

    /// Allocates shared region.
    pub fn allocate_shared_region(
        &mut self,
        module_idx: u32,
        size: u32,
        alignment: u32,
    ) -> Result<SharedRegionId> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.allocate_shared_region(size, alignment)
    }

    /// Destroys shared region.
    pub fn destroy_shared_region(
        &mut self,
        module_idx: u32,
        region_id: SharedRegionId,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.destroy_shared_region(region_id)
    }

    /// Returns the length of the shared region in bytes.
    pub fn shared_region_len(&self, module_idx: u32, region_id: SharedRegionId) -> Result<u32> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.shared_region_len(region_id)
    }

    /// Attaches shared region.
    pub fn attach_shared_region(
        &mut self,
        module_idx: u32,
        region_id: SharedRegionId,
        region_offset: u32,
        len: u32,
    ) -> Result<SharedMemoryMappingId> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.attach_shared_region(region_id, region_offset, len)
    }

    /// Attaches shared region whole.
    pub fn attach_shared_region_whole(
        &mut self,
        module_idx: u32,
        region_id: SharedRegionId,
    ) -> Result<SharedMemoryMappingId> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.attach_shared_region_whole(region_id)
    }

    /// Detaches shared region.
    pub fn detach_shared_region(
        &mut self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.detach_shared_region(mapping_id)
    }

    /// Reads shared region.
    pub fn read_shared_region(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region(mapping_id, offset, buf)
    }

    /// Writes shared region.
    pub fn write_shared_region(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        buf: &[u8],
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region(mapping_id, offset, buf)
    }

    /// Reads shared region u8.
    pub fn read_shared_region_u8(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
    ) -> Result<u8> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region_u8(mapping_id, offset)
    }

    /// Writes shared region u8.
    pub fn write_shared_region_u8(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        value: u8,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region_u8(mapping_id, offset, value)
    }

    /// Reads shared region i32.
    pub fn read_shared_region_i32(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
    ) -> Result<i32> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region_i32(mapping_id, offset)
    }

    /// Writes shared region i32.
    pub fn write_shared_region_i32(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        value: i32,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region_i32(mapping_id, offset, value)
    }

    /// Reads shared region i64.
    pub fn read_shared_region_i64(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
    ) -> Result<i64> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region_i64(mapping_id, offset)
    }

    /// Writes shared region i64.
    pub fn write_shared_region_i64(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        value: i64,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region_i64(mapping_id, offset, value)
    }

    /// Reads shared region f32.
    pub fn read_shared_region_f32(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
    ) -> Result<f32> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region_f32(mapping_id, offset)
    }

    /// Writes shared region f32.
    pub fn write_shared_region_f32(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        value: f32,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region_f32(mapping_id, offset, value)
    }

    /// Reads shared region f64.
    pub fn read_shared_region_f64(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
    ) -> Result<f64> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.read_shared_region_f64(mapping_id, offset)
    }

    /// Writes shared region f64.
    pub fn write_shared_region_f64(
        &self,
        module_idx: u32,
        mapping_id: SharedMemoryMappingId,
        offset: u32,
        value: f64,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.write_shared_region_f64(mapping_id, offset, value)
    }

    /// Calls function.
    pub fn call_function(
        &mut self,
        module_idx: u32,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        #[cfg(feature = "llvm-jit")]
        if matches!(self.execution_mode, ExecutionMode::LlvmJit)
            && self.llvm_jits.contains_key(&module_idx)
        {
            let owner_jit_id = self
                .llvm_jits
                .get(&module_idx)
                .map(LlvmJit::jit_id)
                .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
            let (func_idx, module_ptr, memory_context) = {
                let module = self.runtime.get_module_mut(module_idx).ok_or_else(|| {
                    WasmError::Runtime(format!("module {} not found", module_idx))
                })?;

                let Some(AotExport::Function(func_idx)) = module.get_export(func_name).cloned()
                else {
                    return Err(WasmError::Runtime(format!(
                        "function {} not found",
                        func_name
                    )));
                };

                (func_idx, module as *mut _, module.memory_context())
            };

            let (memory_ptr, memory_len) = memory_context.unwrap_or((std::ptr::null_mut(), 0));
            unsafe {
                set_execution_context(module_ptr, memory_ptr, memory_len, Some(owner_jit_id))
            }?;

            if let Some(llvm_jit) = self.llvm_jits.get_mut(&module_idx) {
                return llvm_jit.invoke_function(func_idx, args);
            }
        }

        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(AotExport::Function(func_idx)) = module.get_export(func_name).cloned() {
            module.invoke_function(func_idx, args)
        } else {
            Err(WasmError::Runtime(format!(
                "function {} not found",
                func_name
            )))
        }
    }

    /// Executes main.
    pub fn execute_main(&mut self, module_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.call_function(module_idx, "main", args)
    }

    /// Executes start.
    pub fn execute_start(&mut self, module_idx: u32) -> Result<()> {
        #[cfg(feature = "llvm-jit")]
        if matches!(self.execution_mode, ExecutionMode::LlvmJit)
            && self.llvm_jits.contains_key(&module_idx)
        {
            let owner_jit_id = self
                .llvm_jits
                .get(&module_idx)
                .map(LlvmJit::jit_id)
                .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
            let (start_idx, module_ptr, memory_context) = {
                let module = self.runtime.get_module_mut(module_idx).ok_or_else(|| {
                    WasmError::Runtime(format!("module {} not found", module_idx))
                })?;
                let Some(start_idx) = module.start_function() else {
                    return Ok(());
                };
                (start_idx, module as *mut _, module.memory_context())
            };

            let (memory_ptr, memory_len) = memory_context.unwrap_or((std::ptr::null_mut(), 0));
            unsafe {
                set_execution_context(module_ptr, memory_ptr, memory_len, Some(owner_jit_id))
            }?;

            if let Some(llvm_jit) = self.llvm_jits.get_mut(&module_idx) {
                let _ = llvm_jit.invoke_function(start_idx, &[])?;
                return Ok(());
            }
        }

        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        if let Some(start_idx) = module.start_function() {
            let _ = module.invoke_function(start_idx, &[])?;
        }

        Ok(())
    }
}

impl Default for WasmApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{FunctionType, Global, GlobalType, NumType, ValType};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct CountingHostFunc;

    impl HostFunc for CountingHostFunc {
        fn call(
            &self,
            store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            let count = store.get_native_func_count() as i32;
            store.register_native(Box::new(NoopHostFunc), FunctionType::empty());
            Ok(vec![WasmValue::I32(count)])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(
                FUNC_TYPE
                    .get_or_init(|| FunctionType::new(vec![], vec![ValType::Num(NumType::I32)])),
            )
        }
    }

    struct NoopHostFunc;

    impl HostFunc for NoopHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            Ok(vec![])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(FUNC_TYPE.get_or_init(FunctionType::empty))
        }
    }

    struct StartHostFunc {
        calls: Arc<AtomicUsize>,
    }

    impl HostFunc for StartHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(FUNC_TYPE.get_or_init(FunctionType::empty))
        }
    }

    #[test]
    fn test_application_creation() {
        let app = WasmApplication::new();
        assert_eq!(app.runtime.modules.len(), 0);
    }

    #[test]
    fn test_load_module_from_memory() {
        let mut app = WasmApplication::new();

        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let result = app.load_module_from_memory(&wasm_data);
        assert!(result.is_ok());

        let idx = result.unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_instantiate() {
        let mut app = WasmApplication::new();

        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        let result = app.instantiate(idx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shared_region_application_wrappers() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        let region_id = app.allocate_shared_region(idx, 16, 8).unwrap();
        let mapping_id = app.attach_shared_region_whole(idx, region_id).unwrap();

        app.write_shared_region_i32(idx, mapping_id, 0, 41).unwrap();
        app.write_shared_region_i32(idx, mapping_id, 4, 59).unwrap();
        assert_eq!(
            app.read_shared_region_i64(idx, mapping_id, 0).unwrap(),
            59i64 << 32 | 41i64
        );

        app.detach_shared_region(idx, mapping_id).unwrap();
        app.destroy_shared_region(idx, region_id).unwrap();
    }

    #[test]
    fn test_instantiate_rejects_missing_imports() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00, 0x00,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        assert!(app.instantiate(idx).is_err());
    }

    #[test]
    fn test_call_function_with_host_import() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00,
            0x00, 0x03, 0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00,
            0x01, 0x0A, 0x06, 0x01, 0x04, 0x00, 0x10, 0x00, 0x0B,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "host",
            Box::new(CountingHostFunc),
            FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
        )
        .unwrap();
        let first = app.call_function(idx, "main", &[]).unwrap();
        let second = app.call_function(idx, "main", &[]).unwrap();

        assert_eq!(first, vec![WasmValue::I32(0)]);
        assert_eq!(second, vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_call_exported_imported_function() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00,
            0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "host",
            Box::new(CountingHostFunc),
            FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
        )
        .unwrap();

        let first = app.call_function(idx, "main", &[]).unwrap();
        let second = app.call_function(idx, "main", &[]).unwrap();

        assert_eq!(first, vec![WasmValue::I32(0)]);
        assert_eq!(second, vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_call_function_with_global_import() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0A, 0x01, 0x03, b'e', b'n', b'v', 0x01, b'g', 0x03, 0x7F, 0x01, 0x03,
            0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00, 0x0A,
            0x0A, 0x01, 0x08, 0x00, 0x41, 0x07, 0x24, 0x00, 0x23, 0x00, 0x0B,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_global_import(
            idx,
            "env",
            "g",
            Global::new(
                GlobalType::new(ValType::Num(NumType::I32), true),
                WasmValue::I32(0),
            )
            .unwrap(),
        )
        .unwrap();

        let results = app.call_function(idx, "main", &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_execute_start_with_imported_function() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'i', b'n', b'i', b't', 0x00, 0x00,
            0x08, 0x01, 0x00,
        ];
        let calls = Arc::new(AtomicUsize::new(0));

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "init",
            Box::new(StartHostFunc {
                calls: calls.clone(),
            }),
            FunctionType::empty(),
        )
        .unwrap();

        app.execute_start(idx).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
