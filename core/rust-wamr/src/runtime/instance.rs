use super::{Global, Import, ImportKind, Memory, Module, Result, Table, WasmError, WasmValue};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Instance {
    module: Arc<Module>,
    memories: Vec<Memory>,
    tables: Vec<Table>,
    globals: Vec<Global>,
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
}

impl<F> HostFunc for F
where
    F: Fn(&mut Store, &[WasmValue]) -> Result<Vec<WasmValue>> + Send + Sync + 'static,
{
    fn call(&self, store: &mut Store, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self(store, args)
    }
}

#[derive(Default)]
pub struct Store {
    pub instances: Vec<Instance>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, instance: Instance) -> usize {
        self.instances.push(instance);
        self.instances.len() - 1
    }
}

impl Instance {
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
    fn test_instance_creation() {
        let module = Arc::new(Module::new());
        let instance = Instance::new(module);
        assert_eq!(instance.funcs.len(), 0);
    }

    #[test]
    fn test_store() {
        let mut store = Store::new();
        let module = Arc::new(Module::new());
        let instance = Instance::new(module);
        let idx = store.add_instance(instance);
        assert_eq!(idx, 0);
    }
}
