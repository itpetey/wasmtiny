use super::loader::AotLoader;
use crate::interpreter::Interpreter;
use crate::runtime::{
    Extern, FunctionType, Global, HostFunc, ImportKind, Instance, Memory, Module, Result, Table,
    WasmError, WasmValue,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct TypedHostImport {
    inner: Arc<dyn HostFunc>,
    func_type: FunctionType,
}

impl TypedHostImport {
    fn new(inner: Arc<dyn HostFunc>, func_type: FunctionType) -> Self {
        Self { inner, func_type }
    }
}

impl HostFunc for TypedHostImport {
    fn call(
        &self,
        store: &mut crate::runtime::Store,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        self.inner.call(store, args)
    }

    fn function_type(&self) -> Option<&FunctionType> {
        Some(&self.func_type)
    }
}

pub struct AotModule {
    module: Module,
    imports: Vec<Option<Extern>>,
    store: Arc<Mutex<crate::runtime::Store>>,
    pub native_functions: Vec<NativeFunc>,
    pub memories: Vec<Memory>,
    pub tables: Vec<Table>,
    pub globals: Vec<Global>,
    pub exports: HashMap<String, AotExport>,
}

impl std::fmt::Debug for AotModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AotModule")
            .field(
                "imports",
                &self
                    .imports
                    .iter()
                    .filter(|binding| binding.is_some())
                    .count(),
            )
            .field("native_functions", &self.native_functions.len())
            .field("memories", &self.memories.len())
            .field("tables", &self.tables)
            .field("globals", &self.globals)
            .field("exports", &self.exports)
            .finish()
    }
}

pub type NativeFunc = Box<dyn Fn(&[WasmValue]) -> Result<Vec<WasmValue>> + Send + Sync>;

#[derive(Debug, Clone)]
pub enum AotExport {
    Function(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

impl AotModule {
    pub fn from_module(module: &Module) -> Self {
        let mut aot_module = Self {
            module: module.clone(),
            imports: vec![None; module.imports.len()],
            store: Arc::new(Mutex::new(crate::runtime::Store::new())),
            native_functions: Vec::new(),
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            exports: HashMap::new(),
        };
        aot_module.initialise_defined_allocations();
        let _ = aot_module.initialise_globals_without_imports();
        aot_module
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn imports(&self) -> &[crate::runtime::Import] {
        &self.module.imports
    }

    pub fn register_native(&mut self, func: NativeFunc) -> u32 {
        let idx = self.native_functions.len() as u32;
        self.native_functions.push(func);
        idx
    }

    pub fn call_native(&self, idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let func = self
            .native_functions
            .get(idx as usize)
            .ok_or_else(|| WasmError::Runtime(format!("native function {} not found", idx)))?;
        func(args)
    }

    pub fn invoke_function(&mut self, idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let (imported_memories, imported_tables, imported_globals) = self.import_counts();
        let imported_funcs = self
            .module
            .imports
            .iter()
            .filter(|import| matches!(import.kind, ImportKind::Func(_)))
            .count() as u32;
        let imports = self.ordered_imports();
        let instance = Arc::new(Mutex::new(Instance::with_imports_and_store(
            Arc::new(self.module.clone()),
            &imports,
            self.store.clone(),
        )?));
        {
            let mut instance_guard = instance.lock().map_err(poisoned_lock)?;
            for (offset, memory) in self.memories.iter().cloned().enumerate() {
                let target = imported_memories + offset;
                if target >= instance_guard.memories.len() {
                    instance_guard.memories.push(Arc::new(Mutex::new(memory)));
                } else {
                    instance_guard.memories[target] = Arc::new(Mutex::new(memory));
                }
            }
            for (offset, table) in self.tables.iter().cloned().enumerate() {
                let target = imported_tables + offset;
                if target >= instance_guard.tables.len() {
                    instance_guard.tables.push(Arc::new(Mutex::new(table)));
                } else {
                    instance_guard.tables[target] = Arc::new(Mutex::new(table));
                }
            }
            for (offset, global) in self.globals.iter().cloned().enumerate() {
                let target = imported_globals + offset;
                if target >= instance_guard.globals.len() {
                    instance_guard.globals.push(Arc::new(Mutex::new(global)));
                } else {
                    instance_guard.globals[target] = Arc::new(Mutex::new(global));
                }
            }
        }

        let results = if idx < imported_funcs {
            instance.lock().map_err(poisoned_lock)?.call(idx, args)?
        } else {
            let mut interpreter = Interpreter::with_instance(instance.clone());
            interpreter.execute_function(&self.module, idx, args)?
        };

        {
            let instance_guard = instance.lock().map_err(poisoned_lock)?;
            self.memories = instance_guard
                .memories
                .iter()
                .skip(imported_memories)
                .map(|memory| {
                    memory
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|memory| memory.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            self.tables = instance_guard
                .tables
                .iter()
                .skip(imported_tables)
                .map(|table| {
                    table
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|table| table.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            self.globals = instance_guard
                .globals
                .iter()
                .skip(imported_globals)
                .map(|global| {
                    global
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|global| global.clone())
                })
                .collect::<Result<Vec<_>>>()?;
        }

        Ok(results)
    }

    pub fn register_import(&mut self, module: &str, name: &str, extern_: Extern) -> Result<()> {
        let matching_indices = self
            .module
            .imports
            .iter()
            .enumerate()
            .filter(|(_, import)| import.module == module && import.name == name)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if matching_indices.is_empty() {
            return Err(WasmError::Instantiate(format!(
                "import {}.{} not found",
                module, name
            )));
        }

        let unresolved = matching_indices
            .into_iter()
            .filter(|idx| self.imports[*idx].is_none())
            .collect::<Vec<_>>();
        if unresolved.is_empty() {
            return Err(WasmError::Instantiate(format!(
                "import {}.{} already registered",
                module, name
            )));
        }

        let mut last_error = None;
        for import_idx in unresolved {
            match self.validate_import_binding(import_idx, module, name, extern_.clone()) {
                Ok(stored) => {
                    self.imports[import_idx] = Some(stored);
                    if self.imports_ready() {
                        self.materialise_defined_state_from_instance()?;
                    }
                    return Ok(());
                }
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            WasmError::Instantiate(format!("import {}.{} kind mismatch", module, name))
        }))
    }

    pub fn register_host_import(
        &mut self,
        module: &str,
        name: &str,
        func: Box<dyn HostFunc>,
        func_type: FunctionType,
    ) -> Result<()> {
        let func: Arc<dyn HostFunc> = Arc::from(func);
        if let Some(actual) = func.function_type()
            && actual != &func_type
        {
            return Err(WasmError::Instantiate(format!(
                "import {}.{} function type mismatch",
                module, name
            )));
        }

        self.register_import(
            module,
            name,
            Extern::HostFunc(Arc::new(TypedHostImport::new(func, func_type))),
        )
    }

    pub fn register_memory_import(
        &mut self,
        module: &str,
        name: &str,
        memory: Memory,
    ) -> Result<()> {
        self.register_import(module, name, Extern::Memory(Arc::new(Mutex::new(memory))))
    }

    pub fn register_table_import(&mut self, module: &str, name: &str, table: Table) -> Result<()> {
        self.register_import(module, name, Extern::Table(Arc::new(Mutex::new(table))))
    }

    pub fn register_global_import(
        &mut self,
        module: &str,
        name: &str,
        global: Global,
    ) -> Result<()> {
        self.register_import(module, name, Extern::Global(Arc::new(Mutex::new(global))))
    }

    pub fn instantiate(&mut self) -> Result<()> {
        self.materialise_defined_state_from_instance()?;
        Ok(())
    }

    pub fn get_export(&self, name: &str) -> Option<&AotExport> {
        self.exports.get(name)
    }

    pub fn start_function(&self) -> Option<u32> {
        self.module.start
    }

    pub fn set_memory(&mut self, memory: Memory) {
        if self.memories.is_empty() {
            self.memories.push(memory);
        } else {
            self.memories[0] = memory;
        }
    }

    pub fn get_memory(&self) -> Option<Memory> {
        if self.import_counts().0 > 0 {
            self.imported_memory(0)
                .and_then(|memory| memory.lock().ok().map(|memory| memory.clone()))
        } else {
            self.memories.first().cloned()
        }
    }

    pub fn memory_context(&mut self) -> Option<(*mut u8, usize)> {
        if self.import_counts().0 > 0 {
            self.imported_memory(0).and_then(|memory| {
                memory
                    .lock()
                    .ok()
                    .map(|mut memory| (memory.as_mut_ptr(), memory.len_bytes()))
            })
        } else {
            self.memories
                .first_mut()
                .map(|memory| (memory.as_mut_ptr(), memory.len_bytes()))
        }
    }

    pub fn add_table(&mut self, table: Table) -> u32 {
        let idx = (self.import_counts().1 + self.tables.len()) as u32;
        self.tables.push(table);
        idx
    }

    pub fn get_table(&self, idx: u32) -> Option<Table> {
        let imported_tables = self.import_counts().1 as u32;
        if idx < imported_tables {
            self.imported_table(idx)
                .and_then(|table| table.lock().ok().map(|table| table.clone()))
        } else {
            self.tables.get((idx - imported_tables) as usize).cloned()
        }
    }

    pub fn add_global(&mut self, global: Global) -> u32 {
        let idx = (self.import_counts().2 + self.globals.len()) as u32;
        self.globals.push(global);
        idx
    }

    pub fn get_global(&self, idx: u32) -> Option<Global> {
        let imported_globals = self.import_counts().2 as u32;
        if idx < imported_globals {
            self.imported_global(idx)
                .and_then(|global| global.lock().ok().map(|global| global.clone()))
        } else {
            self.globals.get((idx - imported_globals) as usize).cloned()
        }
    }

    pub fn get_global_mut(&mut self, idx: u32) -> Option<&mut Global> {
        let imported_globals = self.import_counts().2 as u32;
        if idx < imported_globals {
            None
        } else {
            self.globals.get_mut((idx - imported_globals) as usize)
        }
    }

    pub fn get_func_count(&self) -> u32 {
        self.module.func_count()
    }

    pub fn get_func_type(&self, func_idx: u32) -> Option<&crate::runtime::FunctionType> {
        self.module.func_type(func_idx)
    }

    fn import_counts(&self) -> (usize, usize, usize) {
        let mut memories = 0usize;
        let mut tables = 0usize;
        let mut globals = 0usize;
        for import in &self.module.imports {
            match import.kind {
                crate::runtime::ImportKind::Memory(_) => memories += 1,
                crate::runtime::ImportKind::Table(_) => tables += 1,
                crate::runtime::ImportKind::Global(_) => globals += 1,
                crate::runtime::ImportKind::Func(_) => {}
            }
        }
        (memories, tables, globals)
    }

    fn imports_ready(&self) -> bool {
        self.imports.iter().all(Option::is_some)
    }

    fn initialise_defined_allocations(&mut self) {
        if self.memories.is_empty() {
            self.memories.extend(
                self.module
                    .memories
                    .iter()
                    .cloned()
                    .map(crate::memory::Memory::new),
            );
        }
        if self.tables.is_empty() {
            self.tables
                .extend(self.module.tables.iter().cloned().map(Table::new));
        }
    }

    fn initialise_globals_without_imports(&mut self) -> Result<()> {
        if !self.globals.is_empty()
            || self
                .module
                .imports
                .iter()
                .any(|import| matches!(import.kind, ImportKind::Global(_)))
        {
            return Ok(());
        }

        for (index, global_type) in self.module.globals.iter().enumerate() {
            let init = self.module.global_inits.get(index).ok_or_else(|| {
                WasmError::Instantiate(format!("missing init for global {}", index))
            })?;
            let value = evaluate_const_expr_without_globals(init)?;
            self.globals.push(Global::new(global_type.clone(), value)?);
        }

        Ok(())
    }

    fn materialise_defined_state_from_instance(&mut self) -> Result<()> {
        let (imported_memories, imported_tables, imported_globals) = self.import_counts();
        let imports = self.ordered_imports();
        let instance = Instance::with_imports_and_store(
            Arc::new(self.module.clone()),
            &imports,
            self.store.clone(),
        )?;

        self.memories = instance
            .memories
            .iter()
            .skip(imported_memories)
            .map(|memory| {
                memory
                    .lock()
                    .map_err(poisoned_lock)
                    .map(|memory| memory.clone())
            })
            .collect::<Result<Vec<_>>>()?;
        self.tables = instance
            .tables
            .iter()
            .skip(imported_tables)
            .map(|table| {
                table
                    .lock()
                    .map_err(poisoned_lock)
                    .map(|table| table.clone())
            })
            .collect::<Result<Vec<_>>>()?;
        self.globals = instance
            .globals
            .iter()
            .skip(imported_globals)
            .map(|global| {
                global
                    .lock()
                    .map_err(poisoned_lock)
                    .map(|global| global.clone())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn imported_memory(&self, idx: u32) -> Option<Arc<Mutex<Memory>>> {
        let mut memory_idx = 0u32;
        for (import_idx, import) in self.module.imports.iter().enumerate() {
            if !matches!(import.kind, ImportKind::Memory(_)) {
                continue;
            }
            if memory_idx == idx {
                return match self.imports.get(import_idx)?.as_ref()? {
                    Extern::Memory(memory) => Some(memory.clone()),
                    _ => None,
                };
            }
            memory_idx += 1;
        }
        None
    }

    fn imported_table(&self, idx: u32) -> Option<Arc<Mutex<Table>>> {
        let mut table_idx = 0u32;
        for (import_idx, import) in self.module.imports.iter().enumerate() {
            if !matches!(import.kind, ImportKind::Table(_)) {
                continue;
            }
            if table_idx == idx {
                return match self.imports.get(import_idx)?.as_ref()? {
                    Extern::Table(table) => Some(table.clone()),
                    _ => None,
                };
            }
            table_idx += 1;
        }
        None
    }

    fn imported_global(&self, idx: u32) -> Option<Arc<Mutex<Global>>> {
        let mut global_idx = 0u32;
        for (import_idx, import) in self.module.imports.iter().enumerate() {
            if !matches!(import.kind, ImportKind::Global(_)) {
                continue;
            }
            if global_idx == idx {
                return match self.imports.get(import_idx)?.as_ref()? {
                    Extern::Global(global) => Some(global.clone()),
                    _ => None,
                };
            }
            global_idx += 1;
        }
        None
    }

    fn ordered_imports(&self) -> Vec<(&str, &str, Extern)> {
        self.module
            .imports
            .iter()
            .enumerate()
            .filter_map(|(idx, import)| {
                self.imports[idx]
                    .as_ref()
                    .cloned()
                    .map(|extern_| (import.module.as_str(), import.name.as_str(), extern_))
            })
            .collect()
    }

    fn validate_import_binding(
        &self,
        import_idx: usize,
        module: &str,
        name: &str,
        extern_: Extern,
    ) -> Result<Extern> {
        let import_kind = &self
            .module
            .imports
            .get(import_idx)
            .ok_or_else(|| {
                WasmError::Instantiate(format!("import index {} out of bounds", import_idx))
            })?
            .kind;

        match (import_kind, extern_) {
            (ImportKind::Func(type_idx), Extern::HostFunc(func)) => {
                let func_type = self
                    .module
                    .type_at(*type_idx)
                    .ok_or_else(|| WasmError::Instantiate(format!("type {} not found", type_idx)))?
                    .clone();
                let actual = func.function_type().ok_or_else(|| {
                    WasmError::Instantiate(format!(
                        "import {}.{} host function type is required",
                        module, name
                    ))
                })?;
                if actual != &func_type {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} function type mismatch",
                        module, name
                    )));
                }
                Ok(Extern::HostFunc(Arc::new(TypedHostImport::new(
                    func, func_type,
                ))))
            }
            (ImportKind::Table(expected), Extern::Table(table)) => {
                if !table
                    .lock()
                    .map_err(poisoned_lock)?
                    .type_
                    .matches_required(expected)
                {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} table type mismatch",
                        module, name
                    )));
                }
                Ok(Extern::Table(table))
            }
            (ImportKind::Memory(expected), Extern::Memory(memory)) => {
                if !memory
                    .lock()
                    .map_err(poisoned_lock)?
                    .type_()
                    .matches_required(expected)
                {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} memory type mismatch",
                        module, name
                    )));
                }
                Ok(Extern::Memory(memory))
            }
            (ImportKind::Global(expected), Extern::Global(global)) => {
                if global.lock().map_err(poisoned_lock)?.type_ != *expected {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} global type mismatch",
                        module, name
                    )));
                }
                Ok(Extern::Global(global))
            }
            _ => Err(WasmError::Instantiate(format!(
                "import {}.{} kind mismatch",
                module, name
            ))),
        }
    }
}

pub struct AotRuntime {
    pub modules: Vec<AotModule>,
}

impl AotRuntime {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn load_module(&mut self, data: &[u8]) -> Result<u32> {
        let module = AotLoader::new().load(data)?;
        let module_idx = self.modules.len() as u32;
        self.modules.push(module);
        Ok(module_idx)
    }

    pub fn get_module(&self, idx: u32) -> Option<&AotModule> {
        self.modules.get(idx as usize)
    }

    pub fn get_module_mut(&mut self, idx: u32) -> Option<&mut AotModule> {
        self.modules.get_mut(idx as usize)
    }

    pub fn call(
        &mut self,
        module_idx: u32,
        func_idx: u32,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.invoke_function(func_idx, args)
    }

    pub fn memory_grow(&mut self, module_idx: u32, delta: u32) -> Result<i32> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_memories = module.import_counts().0 as u32;

        if imported_memories > 0 {
            if let Some(memory) = module.imported_memory(0) {
                let mut memory = memory.lock().map_err(poisoned_lock)?;
                let old_size = memory.size();
                memory.grow(delta)?;
                return Ok(old_size as i32);
            }
            return Err(WasmError::Runtime("memory not found".into()));
        }

        if let Some(memory) = module.memories.get_mut(0) {
            let old_size = memory.size();
            memory.grow(delta)?;
            Ok(old_size as i32)
        } else {
            Err(WasmError::Runtime("no memory".into()))
        }
    }

    pub fn memory_size(&self, module_idx: u32) -> Result<i32> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_memories = module.import_counts().0 as u32;

        if imported_memories > 0 {
            if let Some(memory) = module.imported_memory(0) {
                return Ok(memory.lock().map_err(poisoned_lock)?.size() as i32);
            }
            return Err(WasmError::Runtime("memory not found".into()));
        }

        if let Some(memory) = module.memories.first() {
            Ok(memory.size() as i32)
        } else {
            Err(WasmError::Runtime("no memory".into()))
        }
    }

    pub fn table_grow(&mut self, module_idx: u32, table_idx: u32, delta: u32) -> Result<i32> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_tables = module.import_counts().1 as u32;

        if table_idx < imported_tables {
            if let Some(table) = module.imported_table(table_idx) {
                return table
                    .lock()
                    .map_err(poisoned_lock)?
                    .grow(delta)
                    .map(|old_size| old_size as i32)
                    .or(Ok(-1));
            }
            return Err(WasmError::Runtime("table not found".into()));
        }

        if let Some(table) = module
            .tables
            .get_mut((table_idx - imported_tables) as usize)
        {
            table.grow(delta).map(|old_size| old_size as i32).or(Ok(-1))
        } else {
            Err(WasmError::Runtime("table not found".into()))
        }
    }

    pub fn table_size(&self, module_idx: u32, table_idx: u32) -> Result<i32> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_tables = module.import_counts().1 as u32;

        if table_idx < imported_tables {
            if let Some(table) = module.imported_table(table_idx) {
                return Ok(table.lock().map_err(poisoned_lock)?.size() as i32);
            }
            return Err(WasmError::Runtime("table not found".into()));
        }

        if let Some(table) = module.tables.get((table_idx - imported_tables) as usize) {
            Ok(table.size() as i32)
        } else {
            Err(WasmError::Runtime("table not found".into()))
        }
    }

    pub fn get_global_value(&self, module_idx: u32, global_idx: u32) -> Result<WasmValue> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_globals = module.import_counts().2 as u32;

        if global_idx < imported_globals {
            if let Some(global) = module.imported_global(global_idx) {
                return Ok(global.lock().map_err(poisoned_lock)?.get());
            }
            return Err(WasmError::Runtime("global not found".into()));
        }

        if let Some(global) = module.globals.get((global_idx - imported_globals) as usize) {
            Ok(global.value)
        } else {
            Err(WasmError::Runtime("global not found".into()))
        }
    }

    pub fn set_global_value(
        &mut self,
        module_idx: u32,
        global_idx: u32,
        value: WasmValue,
    ) -> Result<()> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        let imported_globals = module.import_counts().2 as u32;

        if global_idx < imported_globals {
            if let Some(global) = module.imported_global(global_idx) {
                return global.lock().map_err(poisoned_lock)?.set(value);
            }
            return Err(WasmError::Runtime("global not found".into()));
        }

        if let Some(global) = module
            .globals
            .get_mut((global_idx - imported_globals) as usize)
        {
            global.set(value)
        } else {
            Err(WasmError::Runtime("global not found".into()))
        }
    }
}

impl Default for AotRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn evaluate_const_expr_without_globals(expr: &[u8]) -> Result<WasmValue> {
    let mut reader = crate::loader::BinaryReader::from_slice(expr);
    let opcode = reader.read_u8().map_err(WasmError::from)?;
    let value = match opcode {
        0x41 => WasmValue::I32(reader.read_sleb128().map_err(WasmError::from)?),
        0x42 => WasmValue::I64(reader.read_sleb128_i64().map_err(WasmError::from)?),
        0x43 => WasmValue::F32(reader.read_f32().map_err(WasmError::from)?),
        0x44 => WasmValue::F64(reader.read_f64().map_err(WasmError::from)?),
        0xD0 => match reader.read_u8().map_err(WasmError::from)? {
            0x70 => WasmValue::NullRef(crate::runtime::RefType::FuncRef),
            0x6F => WasmValue::NullRef(crate::runtime::RefType::ExternRef),
            value => {
                return Err(WasmError::Instantiate(format!(
                    "invalid ref.null type: {:02x}",
                    value
                )));
            }
        },
        0xD2 => WasmValue::FuncRef(reader.read_uleb128().map_err(WasmError::from)?),
        0x23 => {
            return Err(WasmError::Instantiate(
                "global.get requires imported immutable globals to be registered".to_string(),
            ));
        }
        value => {
            return Err(WasmError::Instantiate(format!(
                "unsupported const expr opcode: {:02x}",
                value
            )));
        }
    };

    let end = reader.read_u8().map_err(WasmError::from)?;
    if end != 0x0B {
        return Err(WasmError::Instantiate(
            "constant expression missing end opcode".to_string(),
        ));
    }
    if reader.remaining() != 0 {
        return Err(WasmError::Instantiate(
            "constant expression has trailing bytes".to_string(),
        ));
    }

    Ok(value)
}

fn poisoned_lock<T>(_: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> WasmError {
    WasmError::Runtime("instance lock poisoned".to_string())
}

pub fn create_aot_module_from_wasm(module: &Module) -> AotModule {
    AotModule::from_module(module)
}

pub fn validate_aot_data(data: &[u8]) -> Result<()> {
    let loader = AotLoader::new();
    loader.validate(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RefType;
    use crate::runtime::{GlobalType, Limits, NumType, TableType, ValType};

    struct WrongSigHostFunc;

    impl HostFunc for WrongSigHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            Ok(vec![WasmValue::I32(0)])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(
                FUNC_TYPE
                    .get_or_init(|| FunctionType::new(vec![ValType::Num(NumType::I32)], vec![])),
            )
        }
    }

    struct EmptyHostFunc;

    impl HostFunc for EmptyHostFunc {
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

    struct UntypedHostFunc;

    impl HostFunc for UntypedHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            Ok(vec![])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            None
        }
    }

    #[test]
    fn test_aot_module_creation() {
        let module = AotModule::from_module(&Module::new());
        assert_eq!(module.native_functions.len(), 0);
        assert!(module.memories.is_empty());
    }

    #[test]
    fn test_native_registration() {
        let mut module = AotModule::from_module(&Module::new());
        let idx = module.register_native(Box::new(|_| Ok(vec![])));
        assert_eq!(idx, 0);
        assert_eq!(module.native_functions.len(), 1);
    }

    #[test]
    fn test_native_call() {
        let mut module = AotModule::from_module(&Module::new());
        module.register_native(Box::new(|args| {
            let a = args
                .first()
                .and_then(|v| match v {
                    WasmValue::I32(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            let b = args
                .get(1)
                .and_then(|v| match v {
                    WasmValue::I32(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(vec![WasmValue::I32(a + b)])
        }));

        let result = module
            .call_native(0, &[WasmValue::I32(5), WasmValue::I32(3)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(8)]);
    }

    #[test]
    fn test_get_func_count_reports_module_functions() {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });
        module.funcs.push(crate::runtime::Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });

        let aot_module = AotModule::from_module(&module);
        assert_eq!(aot_module.get_func_count(), 2);
    }

    #[test]
    fn test_table_management() {
        let mut module = AotModule::from_module(&Module::new());
        let table = Table::new(TableType::new(RefType::FuncRef, Limits::Min(10)));
        let idx = module.add_table(table);
        assert_eq!(idx, 0);
        assert!(module.get_table(0).is_some());
    }

    #[test]
    fn test_global_management() {
        let mut module = AotModule::from_module(&Module::new());
        let global = Global::new(
            GlobalType::new(ValType::Num(NumType::I32), true),
            WasmValue::I32(42),
        )
        .unwrap();
        let idx = module.add_global(global);
        assert_eq!(idx, 0);
        assert!(module.get_global(0).is_some());
    }

    #[test]
    fn test_runtime() {
        let runtime = AotRuntime::new();
        assert_eq!(runtime.modules.len(), 0);
    }

    #[test]
    fn test_validate_aot_data() {
        let valid_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert!(validate_aot_data(&valid_data).is_ok());

        let invalid_data = vec![0x00, 0x00, 0x00, 0x00];
        assert!(validate_aot_data(&invalid_data).is_err());

        let short_data = vec![0x00, 0x61];
        assert!(validate_aot_data(&short_data).is_err());

        let truncated_valid_magic = vec![0x00, 0x61, 0x73, 0x6D];
        assert!(validate_aot_data(&truncated_valid_magic).is_err());
    }

    #[test]
    fn test_memory_grow() {
        let mut runtime = AotRuntime::new();
        let module_idx = runtime
            .load_module(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])
            .unwrap();

        {
            let aot_module = runtime.get_module_mut(module_idx).unwrap();
            let mem_type = crate::runtime::MemoryType::new(crate::runtime::Limits::Min(1));
            let memory = crate::memory::Memory::new(mem_type);
            aot_module.set_memory(memory);
        }

        let result = runtime.memory_grow(module_idx, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_size_reads_imported_memory() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(1),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        let mut memory = crate::memory::Memory::new(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));
        memory.grow(1).unwrap();
        aot_module
            .register_memory_import("env", "memory", memory)
            .unwrap();

        let mut runtime = AotRuntime::new();
        runtime.modules.push(aot_module);

        assert_eq!(runtime.memory_size(0).unwrap(), 2);
    }

    #[test]
    fn test_state_is_materialised_after_import_registration() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });
        module.memories.push(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));
        module
            .globals
            .push(GlobalType::new(ValType::Num(NumType::I32), false));
        module.global_inits.push(vec![0x41, 0x07, 0x0B]);

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_host_import(
                "env",
                "host",
                Box::new(EmptyHostFunc),
                FunctionType::empty(),
            )
            .unwrap();

        let mut runtime = AotRuntime::new();
        runtime.modules.push(aot_module);

        assert_eq!(runtime.memory_size(0).unwrap(), 1);
        assert_eq!(runtime.get_global_value(0, 0).unwrap(), WasmValue::I32(7));
    }

    #[test]
    fn test_getters_resolve_imported_state() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(1),
            )),
        });
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: crate::runtime::ImportKind::Table(TableType::new(
                RefType::FuncRef,
                Limits::Min(1),
            )),
        });
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_memory_import(
                "env",
                "memory",
                crate::memory::Memory::new(crate::runtime::MemoryType::new(
                    crate::runtime::Limits::Min(1),
                )),
            )
            .unwrap();
        aot_module
            .register_table_import(
                "env",
                "table",
                Table::new(TableType::new(RefType::FuncRef, Limits::Min(1))),
            )
            .unwrap();
        aot_module
            .register_global_import(
                "env",
                "global",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), false),
                    WasmValue::I32(9),
                )
                .unwrap(),
            )
            .unwrap();

        assert!(aot_module.get_memory().is_some());
        assert!(aot_module.get_table(0).is_some());
        assert_eq!(aot_module.get_global(0).unwrap().get(), WasmValue::I32(9));
    }

    #[test]
    fn test_add_table_returns_combined_index_space() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: crate::runtime::ImportKind::Table(TableType::new(
                RefType::FuncRef,
                Limits::Min(1),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_table_import(
                "env",
                "table",
                Table::new(TableType::new(RefType::FuncRef, Limits::Min(1))),
            )
            .unwrap();

        let idx =
            aot_module.add_table(Table::new(TableType::new(RefType::FuncRef, Limits::Min(2))));

        assert_eq!(idx, 1);
        assert_eq!(aot_module.get_table(idx).unwrap().size(), 2);
    }

    #[test]
    fn test_add_global_returns_combined_index_space() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_global_import(
                "env",
                "global",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), false),
                    WasmValue::I32(9),
                )
                .unwrap(),
            )
            .unwrap();

        let idx = aot_module.add_global(
            Global::new(
                GlobalType::new(ValType::Num(NumType::I32), true),
                WasmValue::I32(42),
            )
            .unwrap(),
        );

        assert_eq!(idx, 1);
        assert_eq!(
            aot_module.get_global(idx).unwrap().get(),
            WasmValue::I32(42)
        );
        assert!(aot_module.get_global_mut(0).is_none());
        assert_eq!(
            aot_module.get_global_mut(idx).unwrap().get(),
            WasmValue::I32(42)
        );
    }

    #[test]
    fn test_invoke_function_preserves_all_memories() {
        let mut module = Module::new();
        module
            .types
            .push(crate::runtime::FunctionType::new(vec![], vec![]));
        module.funcs.push(crate::runtime::Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });
        module.memories.push(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));
        module.memories.push(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));

        let mut aot_module = AotModule::from_module(&module);
        let first = crate::memory::Memory::new(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));
        let mut second = crate::memory::Memory::new(crate::runtime::MemoryType::new(
            crate::runtime::Limits::Min(1),
        ));
        second.write_u8(1, 5).unwrap();
        aot_module.memories = vec![first, second];

        aot_module.invoke_function(0, &[]).unwrap();

        assert_eq!(aot_module.memories.len(), 2);
        assert_eq!(aot_module.memories[1].read_u8(1).unwrap(), 5);
    }

    #[test]
    fn test_imported_global_alias_is_preserved() {
        let mut module = Module::new();
        module
            .types
            .push(crate::runtime::FunctionType::new(vec![], vec![]));
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "g".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                true,
            )),
        });
        module.funcs.push(crate::runtime::Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x2A, 0x24, 0x00, 0x0B],
        });

        let shared = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(NumType::I32), true),
                WasmValue::I32(0),
            )
            .unwrap(),
        ));
        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_import("env", "g", Extern::Global(shared.clone()))
            .unwrap();

        aot_module.invoke_function(0, &[]).unwrap();

        assert_eq!(shared.lock().unwrap().get(), WasmValue::I32(42));
    }

    #[test]
    fn test_set_global_value_respects_imported_global_invariants() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "g".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_global_import(
                "env",
                "g",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), false),
                    WasmValue::I32(1),
                )
                .unwrap(),
            )
            .unwrap();

        let mut runtime = AotRuntime::new();
        runtime.modules.push(aot_module);

        assert_eq!(runtime.get_global_value(0, 0).unwrap(), WasmValue::I32(1));
        assert!(runtime.set_global_value(0, 0, WasmValue::I32(2)).is_err());
        assert!(
            runtime
                .set_global_value(0, 0, WasmValue::FuncRef(0))
                .is_err()
        );
    }

    #[test]
    fn test_register_host_import_rejects_wrong_kind() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(1),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_host_import(
            "env",
            "memory",
            Box::new(EmptyHostFunc),
            FunctionType::empty(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_register_host_import_rejects_wrong_signature() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_host_import(
            "env",
            "host",
            Box::new(WrongSigHostFunc),
            FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_register_host_import_accepts_explicit_signature_for_untyped_host() {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_host_import(
            "env",
            "host",
            Box::new(UntypedHostFunc),
            FunctionType::empty(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_memory_import_rejects_too_small_memory_immediately() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(2),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_memory_import(
            "env",
            "memory",
            crate::memory::Memory::new(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(1),
            )),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_register_memory_import_rejects_broader_max_immediately() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::MinMax(1, 2),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_memory_import(
            "env",
            "memory",
            crate::memory::Memory::new(crate::runtime::MemoryType::new(
                crate::runtime::Limits::MinMax(1, 3),
            )),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_register_memory_and_table_imports_accept_compatible_subtypes() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: crate::runtime::ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::MinMax(1, 4),
            )),
        });
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: crate::runtime::ImportKind::Table(TableType::new(
                RefType::FuncRef,
                crate::runtime::Limits::MinMax(1, 4),
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        assert!(
            aot_module
                .register_memory_import(
                    "env",
                    "memory",
                    crate::memory::Memory::new(crate::runtime::MemoryType::new(
                        crate::runtime::Limits::MinMax(2, 3),
                    )),
                )
                .is_ok()
        );
        assert!(
            aot_module
                .register_table_import(
                    "env",
                    "table",
                    Table::new(TableType::new(
                        RefType::FuncRef,
                        crate::runtime::Limits::MinMax(2, 3),
                    )),
                )
                .is_ok()
        );
    }

    #[test]
    fn test_register_global_import_rejects_wrong_type_immediately() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        let result = aot_module.register_global_import(
            "env",
            "global",
            Global::new(
                GlobalType::new(ValType::Num(NumType::I64), false),
                WasmValue::I64(1),
            )
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_register_import_rejects_duplicate_binding() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                true,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_global_import(
                "env",
                "global",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), true),
                    WasmValue::I32(1),
                )
                .unwrap(),
            )
            .unwrap();

        let error = aot_module
            .register_global_import(
                "env",
                "global",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), true),
                    WasmValue::I32(2),
                )
                .unwrap(),
            )
            .unwrap_err();
        assert!(
            matches!(error, WasmError::Instantiate(message) if message.contains("already registered"))
        );
    }

    #[test]
    fn test_register_duplicate_named_imports_by_occurrence() {
        let mut module = Module::new();
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "shared".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "shared".to_string(),
            kind: crate::runtime::ImportKind::Global(GlobalType::new(
                ValType::Num(NumType::I32),
                false,
            )),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_global_import(
                "env",
                "shared",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), false),
                    WasmValue::I32(1),
                )
                .unwrap(),
            )
            .unwrap();
        aot_module
            .register_global_import(
                "env",
                "shared",
                Global::new(
                    GlobalType::new(ValType::Num(NumType::I32), false),
                    WasmValue::I32(2),
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(aot_module.get_global(0).unwrap().get(), WasmValue::I32(1));
        assert_eq!(aot_module.get_global(1).unwrap().get(), WasmValue::I32(2));
    }

    #[test]
    fn test_invoke_imported_function() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_host_import(
                "env",
                "host",
                Box::new(EmptyHostFunc),
                FunctionType::new(vec![], vec![]),
            )
            .unwrap_err();

        struct ImportedFunc;

        impl HostFunc for ImportedFunc {
            fn call(
                &self,
                _store: &mut crate::runtime::Store,
                _args: &[WasmValue],
            ) -> Result<Vec<WasmValue>> {
                Ok(vec![WasmValue::I32(7)])
            }

            fn function_type(&self) -> Option<&FunctionType> {
                static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
                Some(
                    FUNC_TYPE.get_or_init(|| {
                        FunctionType::new(vec![], vec![ValType::Num(NumType::I32)])
                    }),
                )
            }
        }

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_host_import(
                "env",
                "host",
                Box::new(ImportedFunc),
                FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
            )
            .unwrap();

        let result = aot_module.invoke_function(0, &[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_invoke_imported_function_rejects_result_type_mismatch() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_host_import(
                "env",
                "host",
                Box::new(UntypedHostFunc),
                FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
            )
            .unwrap();

        let error = aot_module.invoke_function(0, &[]).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("result count mismatch"))
        );
    }

    #[test]
    fn test_table_operations() {
        let mut runtime = AotRuntime::new();
        let module_idx = runtime
            .load_module(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])
            .unwrap();

        {
            let aot_module = runtime.get_module_mut(module_idx).unwrap();
            let table = Table::new(TableType::new(RefType::FuncRef, Limits::Min(5)));
            aot_module.add_table(table);
        }

        let size = runtime.table_size(module_idx, 0).unwrap();
        assert_eq!(size, 5);

        let old_size = runtime.table_grow(module_idx, 0, 3).unwrap();
        assert_eq!(old_size, 5);

        let new_size = runtime.table_size(module_idx, 0).unwrap();
        assert_eq!(new_size, 8);
    }

    #[test]
    fn test_global_operations() {
        let mut runtime = AotRuntime::new();
        let module_idx = runtime
            .load_module(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])
            .unwrap();

        {
            let aot_module = runtime.get_module_mut(module_idx).unwrap();
            let global = Global::new(
                GlobalType::new(ValType::Num(NumType::I32), true),
                WasmValue::I32(100),
            )
            .unwrap();
            aot_module.add_global(global);
        }

        let value = runtime.get_global_value(module_idx, 0).unwrap();
        assert_eq!(value, WasmValue::I32(100));

        runtime
            .set_global_value(module_idx, 0, WasmValue::I32(200))
            .unwrap();

        let new_value = runtime.get_global_value(module_idx, 0).unwrap();
        assert_eq!(new_value, WasmValue::I32(200));
    }
}
