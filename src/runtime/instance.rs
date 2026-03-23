use super::{
    ExportKind, FunctionType, Global, ImportKind, Memory, Module, RefType, Result, Table, ValType,
    WasmError, WasmValue,
};
use crate::loader::BinaryReader;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SharedMemory = Arc<Mutex<Memory>>;
pub type SharedTable = Arc<Mutex<Table>>;
pub type SharedGlobal = Arc<Mutex<Global>>;

pub struct Instance {
    module: Arc<Module>,
    store: Arc<Mutex<Store>>,
    pub memories: Vec<SharedMemory>,
    pub tables: Vec<SharedTable>,
    pub globals: Vec<SharedGlobal>,
    funcs: Vec<Arc<dyn HostFunc>>,
    exports: HashMap<String, Extern>,
    import_bindings: Vec<Option<Extern>>,
}

#[derive(Clone)]
pub enum Extern {
    Func(u32),
    HostFunc(Arc<dyn HostFunc>),
    Table(SharedTable),
    Memory(SharedMemory),
    Global(SharedGlobal),
}

impl std::fmt::Debug for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Extern::Func(idx) => f.debug_tuple("Func").field(idx).finish(),
            Extern::HostFunc(_) => f.write_str("HostFunc(..)"),
            Extern::Table(table) => f.debug_tuple("Table").field(table).finish(),
            Extern::Memory(memory) => f.debug_tuple("Memory").field(memory).finish(),
            Extern::Global(global) => f.debug_tuple("Global").field(global).finish(),
        }
    }
}

pub trait HostFunc: Send + Sync + 'static {
    fn call(&self, store: &mut Store, args: &[WasmValue]) -> Result<Vec<WasmValue>>;
    fn function_type(&self) -> Option<&FunctionType>;
}

impl<F> HostFunc for F
where
    F: Fn(&mut Store, &[WasmValue]) -> Result<Vec<WasmValue>> + Send + Sync + 'static,
{
    fn call(&self, store: &mut Store, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self(store, args)
    }

    fn function_type(&self) -> Option<&FunctionType> {
        None
    }
}

pub struct NativeFuncRef {
    pub func: Arc<dyn HostFunc>,
    pub func_type: FunctionType,
    pub name: Option<String>,
}

struct TypedHostFunc {
    inner: Arc<dyn HostFunc>,
    func_type: FunctionType,
}

impl TypedHostFunc {
    fn new(inner: Arc<dyn HostFunc>, func_type: FunctionType) -> Self {
        Self { inner, func_type }
    }
}

impl HostFunc for TypedHostFunc {
    fn call(&self, store: &mut Store, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.inner.call(store, args)
    }

    fn function_type(&self) -> Option<&FunctionType> {
        Some(&self.func_type)
    }
}

impl NativeFuncRef {
    pub fn new(func: Arc<dyn HostFunc>, func_type: FunctionType) -> Self {
        Self {
            func,
            func_type,
            name: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn call(&self, store: &mut Store, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.func.call(store, args)
    }
}

#[derive(Default)]
pub struct Store {
    pub instances: Vec<Instance>,
    native_funcs: Vec<NativeFuncRef>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            native_funcs: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, instance: Instance) -> usize {
        self.instances.push(instance);
        self.instances.len() - 1
    }

    pub fn register_native(&mut self, func: Box<dyn HostFunc>, func_type: FunctionType) -> u32 {
        let idx = self.native_funcs.len() as u32;
        self.native_funcs
            .push(NativeFuncRef::new(Arc::from(func), func_type));
        idx
    }

    pub fn register_native_func(
        &mut self,
        _name: &str,
        func: Box<dyn HostFunc>,
        func_type: FunctionType,
    ) -> u32 {
        self.register_native(func, func_type)
    }

    pub fn get_native_func(&self, idx: u32) -> Option<&NativeFuncRef> {
        self.native_funcs.get(idx as usize)
    }

    pub fn get_native_func_count(&self) -> u32 {
        self.native_funcs.len() as u32
    }
}

impl Instance {
    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn new(module: Arc<Module>) -> Result<Self> {
        Self::new_with_store(module, Arc::new(Mutex::new(Store::new())))
    }

    pub fn new_with_store(module: Arc<Module>, store: Arc<Mutex<Store>>) -> Result<Self> {
        let mut instance = Self::empty(module, store);
        instance.instantiate_host_funcs();
        instance.validate_imports_satisfied()?;
        instance.instantiate_defined_state()?;
        instance.instantiate_exports();
        Ok(instance)
    }

    pub fn with_imports(module: Arc<Module>, imports: &[(&str, &str, Extern)]) -> Result<Self> {
        Self::with_imports_and_store(module, imports, Arc::new(Mutex::new(Store::new())))
    }

    pub fn with_imports_and_store(
        module: Arc<Module>,
        imports: &[(&str, &str, Extern)],
        store: Arc<Mutex<Store>>,
    ) -> Result<Self> {
        let mut instance = Self::empty(module, store);
        instance.instantiate_host_funcs();
        let mut used = vec![false; imports.len()];
        let import_decls = instance.module.imports.clone();
        for (import_idx, import) in import_decls.iter().enumerate() {
            if let Some((provided_idx, (_, _, extern_))) =
                imports
                    .iter()
                    .enumerate()
                    .find(|(provided_idx, (module_name, name, _))| {
                        !used[*provided_idx]
                            && *module_name == import.module.as_str()
                            && *name == import.name.as_str()
                    })
            {
                used[provided_idx] = true;
                instance.add_import_at(import_idx, extern_)?;
            }
        }
        instance.validate_imports_satisfied()?;
        instance.instantiate_defined_state()?;
        instance.instantiate_exports();
        Ok(instance)
    }

    fn empty(module: Arc<Module>, store: Arc<Mutex<Store>>) -> Self {
        let import_count = module.imports.len();
        Self {
            module,
            store,
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            funcs: Vec::new(),
            exports: HashMap::new(),
            import_bindings: vec![None; import_count],
        }
    }

    fn instantiate_host_funcs(&mut self) {
        let import_func_count = self
            .module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Func(_)))
            .count();

        self.funcs = (0..import_func_count)
            .map(|_| {
                Arc::new(|_: &mut Store, _: &[WasmValue]| {
                    Err(WasmError::Runtime(
                        "uninitialized host function".to_string(),
                    ))
                }) as Arc<dyn HostFunc>
            })
            .collect();
    }

    fn instantiate_defined_state(&mut self) -> Result<()> {
        let const_globals = self.const_expr_globals();

        for memory_type in &self.module.memories {
            self.memories
                .push(Arc::new(Mutex::new(Memory::new(memory_type.clone()))));
        }

        for table_type in &self.module.tables {
            self.tables
                .push(Arc::new(Mutex::new(Table::new(table_type.clone()))));
        }

        for (index, global_type) in self.module.globals.iter().enumerate() {
            let init = self.module.global_inits.get(index).ok_or_else(|| {
                WasmError::Instantiate(format!("missing init for global {}", index))
            })?;
            let value = evaluate_const_expr(init, &const_globals)?;
            self.globals.push(Arc::new(Mutex::new(Global::new(
                global_type.clone(),
                value,
            )?)));
        }

        self.initialise_data_segments(&const_globals)?;
        self.initialise_elem_segments(&const_globals)?;

        Ok(())
    }

    fn const_expr_globals(&self) -> Vec<Option<SharedGlobal>> {
        let mut globals = Vec::new();
        let mut imported_global_idx = 0usize;

        for import in &self.module.imports {
            if let ImportKind::Global(global_type) = &import.kind {
                let global = self.globals.get(imported_global_idx).cloned();
                imported_global_idx += 1;

                if global_type.mutable {
                    globals.push(None);
                } else {
                    globals.push(global);
                }
            }
        }

        globals
    }

    fn validate_imports_satisfied(&self) -> Result<()> {
        for (index, (import, binding)) in self
            .module
            .imports
            .iter()
            .zip(self.import_bindings.iter())
            .enumerate()
        {
            if binding.is_none() {
                return Err(WasmError::Instantiate(format!(
                    "import {}.{} at index {} is not satisfied",
                    import.module, import.name, index
                )));
            }
        }

        Ok(())
    }

    fn initialise_data_segments(&mut self, const_globals: &[Option<SharedGlobal>]) -> Result<()> {
        for segment in &self.module.data {
            let super::DataKind::Active { memory_idx, offset } = &segment.kind else {
                continue;
            };
            let offset = evaluate_const_expr(offset, const_globals)?;
            let WasmValue::I32(offset) = offset else {
                return Err(WasmError::Instantiate(
                    "data segment offset must evaluate to i32".to_string(),
                ));
            };

            let memory = self.memories.get_mut(*memory_idx as usize).ok_or_else(|| {
                WasmError::Instantiate(format!("memory {} not found", memory_idx))
            })?;
            memory
                .lock()
                .map_err(poisoned_lock)?
                .write(offset as u32, &segment.init)?;
        }

        Ok(())
    }

    fn initialise_elem_segments(&mut self, const_globals: &[Option<SharedGlobal>]) -> Result<()> {
        for segment in &self.module.elems {
            let super::ElemKind::Active { table_idx, offset } = &segment.kind else {
                continue;
            };
            let offset = evaluate_const_expr(offset, const_globals)?;
            let WasmValue::I32(offset) = offset else {
                return Err(WasmError::Instantiate(
                    "element segment offset must evaluate to i32".to_string(),
                ));
            };

            let table = self
                .tables
                .get_mut(*table_idx as usize)
                .ok_or_else(|| WasmError::Instantiate(format!("table {} not found", table_idx)))?;

            for (index, expr) in segment.init.iter().enumerate() {
                let value = evaluate_const_expr(expr, const_globals)?;
                table
                    .lock()
                    .map_err(poisoned_lock)?
                    .set(offset as u32 + index as u32, value)?;
            }
        }

        Ok(())
    }

    fn instantiate_exports(&mut self) {
        for export in &self.module.exports {
            let extern_ = match export.kind {
                ExportKind::Func(idx) => Some(Extern::Func(idx)),
                ExportKind::Table(idx) => self.tables.get(idx as usize).cloned().map(Extern::Table),
                ExportKind::Memory(idx) => {
                    self.memories.get(idx as usize).cloned().map(Extern::Memory)
                }
                ExportKind::Global(idx) => {
                    self.globals.get(idx as usize).cloned().map(Extern::Global)
                }
            };

            if let Some(extern_) = extern_ {
                self.exports.insert(export.name.clone(), extern_);
            }
        }
    }

    pub fn add_import(&mut self, module_name: &str, name: &str, extern_: &Extern) -> Result<()> {
        let matching_indices = self
            .module
            .imports
            .iter()
            .enumerate()
            .filter(|(_, import)| import.module == module_name && import.name == name)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        if matching_indices.is_empty() {
            return Err(WasmError::Instantiate(format!(
                "import {}.{} not found",
                module_name, name
            )));
        }

        let unresolved = matching_indices
            .into_iter()
            .filter(|idx| self.import_bindings[*idx].is_none())
            .collect::<Vec<_>>();
        if unresolved.is_empty() {
            return Err(WasmError::Instantiate(format!(
                "import {}.{} already registered",
                module_name, name
            )));
        }

        let mut last_error = None;
        for import_idx in unresolved {
            match self.add_import_at(import_idx, extern_) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            WasmError::Instantiate(format!("import {}.{} kind mismatch", module_name, name))
        }))
    }

    pub fn get_func(&self, idx: u32) -> Option<&dyn HostFunc> {
        self.funcs.get(idx as usize).map(|f| f.as_ref())
    }

    pub fn memory(&self, idx: u32) -> Option<&SharedMemory> {
        self.memories.get(idx as usize)
    }

    pub fn memory_mut(&mut self, idx: u32) -> Option<&mut SharedMemory> {
        self.memories.get_mut(idx as usize)
    }

    pub fn table(&self, idx: u32) -> Option<&SharedTable> {
        self.tables.get(idx as usize)
    }

    pub fn table_mut(&mut self, idx: u32) -> Option<&mut SharedTable> {
        self.tables.get_mut(idx as usize)
    }

    pub fn global(&self, idx: u32) -> Option<&SharedGlobal> {
        self.globals.get(idx as usize)
    }

    pub fn global_mut(&mut self, idx: u32) -> Option<&mut SharedGlobal> {
        self.globals.get_mut(idx as usize)
    }

    pub fn call(&mut self, func_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        if let Some(func) = self.get_func(func_idx) {
            let func_type = func.function_type().ok_or_else(|| {
                WasmError::Runtime(format!("function {} type not found", func_idx))
            })?;
            validate_values(args, &func_type.params, "argument")?;
            let mut store = self.store.lock().map_err(poisoned_lock)?;
            let results = func.call(&mut store, args)?;
            validate_values(&results, &func_type.results, "result")?;
            Ok(results)
        } else {
            Err(WasmError::Runtime(format!(
                "function {} not found",
                func_idx
            )))
        }
    }

    pub fn export(&self, name: &str) -> Option<&Extern> {
        self.exports.get(name)
    }

    pub fn add_export(&mut self, name: String, extern_: Extern) {
        self.exports.insert(name, extern_);
    }

    fn add_import_at(&mut self, import_idx: usize, extern_: &Extern) -> Result<()> {
        let import = self.module.imports.get(import_idx).ok_or_else(|| {
            WasmError::Instantiate(format!("import index {} out of bounds", import_idx))
        })?;

        match (&import.kind, extern_) {
            (ImportKind::Func(type_idx), Extern::HostFunc(func)) => {
                let expected = self.module.type_at(*type_idx).ok_or_else(|| {
                    WasmError::Instantiate(format!("type {} not found", type_idx))
                })?;
                if let Some(actual) = func.function_type()
                    && actual != expected
                {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} function type mismatch",
                        import.module, import.name
                    )));
                }
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
                        import.module, import.name
                    )));
                }
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
                        import.module, import.name
                    )));
                }
            }
            (ImportKind::Global(expected), Extern::Global(global)) => {
                if global.lock().map_err(poisoned_lock)?.type_ != *expected {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} global type mismatch",
                        import.module, import.name
                    )));
                }
            }
            _ => {
                return Err(WasmError::Instantiate(format!(
                    "import {}.{} kind mismatch",
                    import.module, import.name
                )));
            }
        }

        self.import_bindings[import_idx] = Some(extern_.clone());
        self.sync_import_bindings()
    }

    fn sync_import_bindings(&mut self) -> Result<()> {
        self.instantiate_host_funcs();
        self.tables.clear();
        self.memories.clear();
        self.globals.clear();

        for (import_idx, import) in self.module.imports.iter().enumerate() {
            let Some(binding) = self.import_bindings[import_idx].as_ref() else {
                continue;
            };

            match (&import.kind, binding) {
                (ImportKind::Func(type_idx), Extern::HostFunc(func)) => {
                    let expected = self
                        .module
                        .type_at(*type_idx)
                        .ok_or_else(|| {
                            WasmError::Instantiate(format!("type {} not found", type_idx))
                        })?
                        .clone();
                    let slot = self.func_import_slot(import_idx);
                    self.funcs[slot] = Arc::new(TypedHostFunc::new(func.clone(), expected));
                }
                (ImportKind::Table(_), Extern::Table(table)) => self.tables.push(table.clone()),
                (ImportKind::Memory(_), Extern::Memory(memory)) => {
                    self.memories.push(memory.clone())
                }
                (ImportKind::Global(_), Extern::Global(global)) => {
                    self.globals.push(global.clone())
                }
                _ => {
                    return Err(WasmError::Instantiate(format!(
                        "import {}.{} kind mismatch",
                        import.module, import.name
                    )));
                }
            }
        }

        Ok(())
    }

    fn func_import_slot(&self, import_idx: usize) -> usize {
        self.module.imports[..import_idx]
            .iter()
            .filter(|import| matches!(import.kind, ImportKind::Func(_)))
            .count()
    }
}

fn evaluate_const_expr(expr: &[u8], globals: &[Option<SharedGlobal>]) -> Result<WasmValue> {
    let mut reader = BinaryReader::from_slice(expr);
    let opcode = reader.read_u8().map_err(io_to_load_error)?;
    let value = match opcode {
        0x23 => {
            let idx = reader.read_uleb128().map_err(io_to_load_error)?;
            globals
                .get(idx as usize)
                .and_then(|global| global.as_ref())
                .ok_or_else(|| {
                    WasmError::Instantiate(format!(
                        "global {} is not allowed in constant expressions",
                        idx
                    ))
                })?
                .lock()
                .map_err(poisoned_lock)?
                .get()
        }
        0x41 => WasmValue::I32(reader.read_sleb128().map_err(io_to_load_error)?),
        0x42 => WasmValue::I64(reader.read_sleb128_i64().map_err(io_to_load_error)?),
        0x43 => WasmValue::F32(reader.read_f32().map_err(io_to_load_error)?),
        0x44 => WasmValue::F64(reader.read_f64().map_err(io_to_load_error)?),
        0xD0 => match reader.read_u8().map_err(io_to_load_error)? {
            0x70 => WasmValue::NullRef(RefType::FuncRef),
            0x6F => WasmValue::NullRef(RefType::ExternRef),
            value => {
                return Err(WasmError::Instantiate(format!(
                    "invalid ref.null type: {:02x}",
                    value
                )));
            }
        },
        0xD2 => WasmValue::FuncRef(reader.read_uleb128().map_err(io_to_load_error)?),
        value => {
            return Err(WasmError::Instantiate(format!(
                "unsupported const expr opcode: {:02x}",
                value
            )));
        }
    };

    let end = reader.read_u8().map_err(io_to_load_error)?;
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

fn io_to_load_error(error: std::io::Error) -> WasmError {
    WasmError::Load(error.to_string())
}

fn validate_values(values: &[WasmValue], expected: &[ValType], kind: &str) -> Result<()> {
    if values.len() != expected.len() {
        return Err(WasmError::Runtime(format!(
            "{} count mismatch: expected {}, got {}",
            kind,
            expected.len(),
            values.len()
        )));
    }

    for (index, (value, expected_type)) in values.iter().zip(expected.iter()).enumerate() {
        if value.val_type() != *expected_type {
            return Err(WasmError::Runtime(format!(
                "{} {} type mismatch: expected {:?}, got {:?}",
                kind,
                index,
                expected_type,
                value.val_type()
            )));
        }
    }

    Ok(())
}

fn poisoned_lock<T>(_: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> WasmError {
    WasmError::Runtime("instance lock poisoned".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Func, GlobalType, Import, Limits, MemoryType, TableType, ValType};

    #[test]
    fn test_store() {
        let mut store = Store::new();
        let module = Arc::new(Module::new());
        let instance = Instance::new(module).unwrap();
        let idx = store.add_instance(instance);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_native_func_registration() {
        let mut store = Store::new();
        let func_type = FunctionType::new(
            vec![
                ValType::Num(crate::runtime::NumType::I32),
                ValType::Num(crate::runtime::NumType::I32),
            ],
            vec![ValType::Num(crate::runtime::NumType::I32)],
        );

        let func: Box<dyn HostFunc> = Box::new(|_store: &mut Store, args: &[WasmValue]| {
            let a = args[0].i32()?;
            let b = args[1].i32()?;
            Ok(vec![WasmValue::I32(a + b)])
        });

        let idx = store.register_native(func, func_type);
        assert_eq!(idx, 0);
        assert_eq!(store.get_native_func_count(), 1);
    }

    #[test]
    fn test_imported_state_is_shared() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryType::new(Limits::Min(1))),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: ImportKind::Table(TableType::new(RefType::FuncRef, Limits::Min(1))),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: ImportKind::Global(GlobalType::new(
                ValType::Num(crate::runtime::NumType::I32),
                true,
            )),
        });

        let memory = Arc::new(Mutex::new(Memory::new(MemoryType::new(Limits::Min(1)))));
        let table = Arc::new(Mutex::new(Table::new(TableType::new(
            RefType::FuncRef,
            Limits::Min(1),
        ))));
        let global = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(crate::runtime::NumType::I32), true),
                WasmValue::I32(1),
            )
            .unwrap(),
        ));

        let mut instance = Instance::with_imports(
            Arc::new(module),
            &[
                ("env", "memory", Extern::Memory(memory.clone())),
                ("env", "table", Extern::Table(table.clone())),
                ("env", "global", Extern::Global(global.clone())),
            ],
        )
        .unwrap();

        instance
            .memory_mut(0)
            .unwrap()
            .lock()
            .unwrap()
            .write_u8(0, 9)
            .unwrap();
        instance
            .table_mut(0)
            .unwrap()
            .lock()
            .unwrap()
            .set(0, WasmValue::FuncRef(7))
            .unwrap();
        instance
            .global_mut(0)
            .unwrap()
            .lock()
            .unwrap()
            .set(WasmValue::I32(42))
            .unwrap();

        assert_eq!(memory.lock().unwrap().read_u8(0).unwrap(), 9);
        assert_eq!(table.lock().unwrap().get(0), Some(WasmValue::FuncRef(7)));
        assert_eq!(global.lock().unwrap().get(), WasmValue::I32(42));
    }

    #[test]
    fn test_missing_imports_fail_instantiation() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryType::new(Limits::Min(1))),
        });

        let result = Instance::with_imports(Arc::new(module), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_imports_are_bound_in_module_order() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "first".to_string(),
            kind: ImportKind::Global(GlobalType::new(
                ValType::Num(crate::runtime::NumType::I32),
                false,
            )),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "second".to_string(),
            kind: ImportKind::Global(GlobalType::new(
                ValType::Num(crate::runtime::NumType::I32),
                false,
            )),
        });
        module.types.push(FunctionType::new(
            vec![],
            vec![ValType::Num(crate::runtime::NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x23, 0x00, 0x0B],
        });

        let first = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(crate::runtime::NumType::I32), false),
                WasmValue::I32(7),
            )
            .unwrap(),
        ));
        let second = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(crate::runtime::NumType::I32), false),
                WasmValue::I32(11),
            )
            .unwrap(),
        ));

        let instance = Arc::new(Mutex::new(
            Instance::with_imports(
                Arc::new(module.clone()),
                &[
                    ("env", "second", Extern::Global(second)),
                    ("env", "first", Extern::Global(first)),
                ],
            )
            .unwrap(),
        ));

        let mut interpreter = crate::interpreter::Interpreter::with_instance(instance);
        let results = interpreter.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_memory_and_table_imports_accept_compatible_subtypes() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryType::new(Limits::MinMax(1, 4))),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: ImportKind::Table(TableType::new(RefType::FuncRef, Limits::MinMax(1, 4))),
        });

        let memory = Arc::new(Mutex::new(Memory::new(MemoryType::new(Limits::MinMax(
            2, 3,
        )))));
        let table = Arc::new(Mutex::new(Table::new(TableType::new(
            RefType::FuncRef,
            Limits::MinMax(2, 3),
        ))));

        let instance = Instance::with_imports(
            Arc::new(module),
            &[
                ("env", "memory", Extern::Memory(memory)),
                ("env", "table", Extern::Table(table)),
            ],
        );

        assert!(instance.is_ok());
    }

    #[test]
    fn test_with_imports_accepts_untyped_host_func() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(crate::runtime::NumType::I32)],
            vec![ValType::Num(crate::runtime::NumType::I32)],
        ));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });

        let mut instance = Instance::with_imports(
            Arc::new(module),
            &[(
                "env",
                "host",
                Extern::HostFunc(Arc::new(|_: &mut Store, args: &[WasmValue]| {
                    Ok(vec![args[0]])
                })),
            )],
        )
        .unwrap();

        let results = instance.call(0, &[WasmValue::I32(9)]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(9)]);
    }

    #[test]
    fn test_host_call_rejects_argument_type_mismatch() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(crate::runtime::NumType::I32)],
            vec![],
        ));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });

        let mut instance = Instance::with_imports(
            Arc::new(module),
            &[(
                "env",
                "host",
                Extern::HostFunc(Arc::new(|_: &mut Store, _: &[WasmValue]| Ok(vec![]))),
            )],
        )
        .unwrap();

        let error = instance.call(0, &[WasmValue::F64(1.0)]).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("argument 0 type mismatch"))
        );
    }

    #[test]
    fn test_host_call_rejects_result_type_mismatch() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![],
            vec![ValType::Num(crate::runtime::NumType::I32)],
        ));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });

        let mut instance = Instance::with_imports(
            Arc::new(module),
            &[(
                "env",
                "host",
                Extern::HostFunc(Arc::new(|_: &mut Store, _: &[WasmValue]| Ok(vec![]))),
            )],
        )
        .unwrap();

        let error = instance.call(0, &[]).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("result count mismatch"))
        );
    }

    #[test]
    fn test_duplicate_named_imports_bind_by_occurrence() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "shared".to_string(),
            kind: ImportKind::Global(GlobalType::new(
                ValType::Num(crate::runtime::NumType::I32),
                false,
            )),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "shared".to_string(),
            kind: ImportKind::Global(GlobalType::new(
                ValType::Num(crate::runtime::NumType::I32),
                false,
            )),
        });

        let first = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(crate::runtime::NumType::I32), false),
                WasmValue::I32(1),
            )
            .unwrap(),
        ));
        let second = Arc::new(Mutex::new(
            Global::new(
                GlobalType::new(ValType::Num(crate::runtime::NumType::I32), false),
                WasmValue::I32(2),
            )
            .unwrap(),
        ));

        let instance = Instance::with_imports(
            Arc::new(module),
            &[
                ("env", "shared", Extern::Global(first)),
                ("env", "shared", Extern::Global(second)),
            ],
        )
        .unwrap();

        assert_eq!(
            instance.global(0).unwrap().lock().unwrap().get(),
            WasmValue::I32(1)
        );
        assert_eq!(
            instance.global(1).unwrap().lock().unwrap().get(),
            WasmValue::I32(2)
        );
    }
}
