use super::{
    FunctionType, Global, ImportKind, Memory, Module, Result, Table, WasmError, WasmValue,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Instance {
    module: Arc<Module>,
    pub memories: Vec<Memory>,
    pub tables: Vec<Table>,
    pub globals: Vec<Global>,
    funcs: Vec<Box<dyn HostFunc>>,
    exports: HashMap<String, Extern>,
}

#[derive(Debug, Clone)]
pub enum Extern {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
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
    pub func: Box<dyn HostFunc>,
    pub func_type: FunctionType,
    pub name: Option<String>,
}

impl NativeFuncRef {
    pub fn new(func: Box<dyn HostFunc>, func_type: FunctionType) -> Self {
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
        self.native_funcs.push(NativeFuncRef::new(func, func_type));
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

    pub fn new(module: Arc<Module>) -> Self {
        let mut instance = Self {
            module,
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            funcs: Vec::new(),
            exports: HashMap::new(),
        };
        instance.instantiate_host_funcs();
        instance
    }

    pub fn with_imports(module: Arc<Module>, imports: &[(&str, &str, Extern)]) -> Result<Self> {
        let mut instance = Self::new(module);
        for (module_name, name, extern_) in imports {
            instance.add_import(module_name, name, extern_)?;
        }
        Ok(instance)
    }

    fn instantiate_host_funcs(&mut self) {
        let import_func_count = self
            .module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Func(_)))
            .count();

        self.funcs = (0..import_func_count)
            .map(|_| -> Box<dyn HostFunc> {
                Box::new(|_: &mut Store, _: &[WasmValue]| {
                    Err(WasmError::Runtime(
                        "uninitialized host function".to_string(),
                    ))
                })
            })
            .collect();
    }

    pub fn add_import(&mut self, _module_name: &str, _name: &str, _extern: &Extern) -> Result<()> {
        Ok(())
    }

    pub fn get_func(&self, idx: u32) -> Option<&dyn HostFunc> {
        self.funcs.get(idx as usize).map(|f| f.as_ref())
    }

    pub fn memory(&self, idx: u32) -> Option<&Memory> {
        self.memories.get(idx as usize)
    }

    pub fn memory_mut(&mut self, idx: u32) -> Option<&mut Memory> {
        self.memories.get_mut(idx as usize)
    }

    pub fn table(&self, idx: u32) -> Option<&Table> {
        self.tables.get(idx as usize)
    }

    pub fn table_mut(&mut self, idx: u32) -> Option<&mut Table> {
        self.tables.get_mut(idx as usize)
    }

    pub fn global(&self, idx: u32) -> Option<&Global> {
        self.globals.get(idx as usize)
    }

    pub fn global_mut(&mut self, idx: u32) -> Option<&mut Global> {
        self.globals.get_mut(idx as usize)
    }

    pub fn call(&mut self, func_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        if let Some(func) = self.get_func(func_idx) {
            let mut store = Store::new();
            func.call(&mut store, args)
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
}

unsafe impl Send for Instance {}
unsafe impl Sync for Instance {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store() {
        let mut store = Store::new();
        let module = Arc::new(Module::new());
        let instance = Instance::new(module);
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
            let a = args[0].i32();
            let b = args[1].i32();
            Ok(vec![WasmValue::I32(a + b)])
        });

        let idx = store.register_native(func, func_type);
        assert_eq!(idx, 0);
        assert_eq!(store.get_native_func_count(), 1);
    }
}
