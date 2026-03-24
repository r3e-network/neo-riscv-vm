use crate::bridge::{register_host_functions, ClosureHost};
use polkavm::{
    BackendKind as PolkaBackendKind, Config, Engine, Instance, InstancePre, Linker, Module,
    ModuleConfig, ProgramBlob,
};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();
static GUEST_BLOB: OnceLock<Result<ProgramBlob, String>> = OnceLock::new();
static MODULES: OnceLock<Mutex<HashMap<u32, Module>>> = OnceLock::new();
type CachedInstancePre = InstancePre<ClosureHost, core::convert::Infallible>;
type ExecutionInstance = Instance<ClosureHost, core::convert::Infallible>;
type InstancePreMap = HashMap<u32, CachedInstancePre>;
type ExecutionInstancePool = HashMap<u32, Vec<ExecutionInstance>>;

static INSTANCE_PRES: OnceLock<Mutex<InstancePreMap>> = OnceLock::new();
static EXECUTION_INSTANCES: OnceLock<Mutex<ExecutionInstancePool>> = OnceLock::new();

pub(crate) struct CachedExecutionInstance {
    aux_size: u32,
    instance_pre: CachedInstancePre,
    instance: Option<ExecutionInstance>,
}

impl CachedExecutionInstance {
    pub(crate) fn module(&self) -> &Module {
        self.instance_pre.module()
    }

    pub(crate) fn instance_mut(&mut self) -> &mut ExecutionInstance {
        self.instance
            .as_mut()
            .expect("cached execution instance should be present")
    }
}

impl Drop for CachedExecutionInstance {
    fn drop(&mut self) {
        let Some(instance) = self.instance.take() else {
            return;
        };

        if let Some(pool) = EXECUTION_INSTANCES.get() {
            if let Ok(mut guard) = pool.lock() {
                guard.entry(self.aux_size).or_default().push(instance);
            }
        }
    }
}

pub(crate) fn ensure_runtime_ready() -> Result<(), String> {
    let _ = cached_engine()?;
    let _ = guest_blob()?;
    Ok(())
}

pub(crate) fn cached_module(aux_size: u32) -> Result<Module, String> {
    let modules = MODULES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(module) = modules
        .lock()
        .map_err(|_| "polkavm module cache poisoned".to_string())?
        .get(&aux_size)
        .cloned()
    {
        return Ok(module);
    }

    let engine = cached_engine()?;
    let blob = guest_blob()?.clone();
    let mut module_config = ModuleConfig::new();
    if aux_size > 0 {
        module_config.set_aux_data_size(aux_size);
    }
    let module = Module::from_blob(engine, &module_config, blob).map_err(|e| e.to_string())?;

    let mut guard = modules
        .lock()
        .map_err(|_| "polkavm module cache poisoned".to_string())?;
    Ok(guard
        .entry(aux_size)
        .or_insert_with(|| module.clone())
        .clone())
}

pub(crate) fn cached_instance_pre(aux_size: u32) -> Result<CachedInstancePre, String> {
    let instance_pres = INSTANCE_PRES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(instance_pre) = instance_pres
        .lock()
        .map_err(|_| "polkavm instance-pre cache poisoned".to_string())?
        .get(&aux_size)
        .cloned()
    {
        return Ok(instance_pre);
    }

    let module = cached_module(aux_size)?;
    let mut linker = Linker::<ClosureHost, core::convert::Infallible>::new();
    register_host_functions(&mut linker)?;
    let instance_pre = linker.instantiate_pre(&module).map_err(|e| e.to_string())?;

    let mut guard = instance_pres
        .lock()
        .map_err(|_| "polkavm instance-pre cache poisoned".to_string())?;
    Ok(guard
        .entry(aux_size)
        .or_insert_with(|| instance_pre.clone())
        .clone())
}

pub(crate) fn cached_execution_instance(aux_size: u32) -> Result<CachedExecutionInstance, String> {
    let instance_pre = cached_instance_pre(aux_size)?;
    let pool = EXECUTION_INSTANCES.get_or_init(|| Mutex::new(HashMap::new()));

    let mut instance = {
        let mut guard = pool
            .lock()
            .map_err(|_| "polkavm execution-instance pool poisoned".to_string())?;
        guard
            .get_mut(&aux_size)
            .and_then(|instances| instances.pop())
    }
    .unwrap_or(instance_pre.instantiate().map_err(|e| e.to_string())?);

    instance.reset_memory().map_err(|e| e.to_string())?;

    Ok(CachedExecutionInstance {
        aux_size,
        instance_pre,
        instance: Some(instance),
    })
}

pub(crate) fn compile_native_module(binary: &[u8], aux_size: u32) -> Result<Module, String> {
    let engine = cached_engine()?;
    let blob = ProgramBlob::parse(binary.into()).map_err(|e| e.to_string())?;
    let mut module_config = ModuleConfig::new();
    if aux_size > 0 {
        module_config.set_aux_data_size(aux_size);
    }
    Module::from_blob(engine, &module_config, blob).map_err(|e| e.to_string())
}

fn cached_engine() -> Result<&'static Engine, String> {
    match ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.set_backend(Some(PolkaBackendKind::Interpreter));
        Engine::new(&config).map_err(|error| error.to_string())
    }) {
        Ok(engine) => Ok(engine),
        Err(error) => Err(error.clone()),
    }
}

fn guest_blob() -> Result<&'static ProgramBlob, String> {
    match GUEST_BLOB.get_or_init(|| {
        ProgramBlob::parse(
            include_bytes!("../../../crates/neo-riscv-guest-module/guest.polkavm")
                .as_ref()
                .into(),
        )
        .map_err(|e| e.to_string())
    }) {
        Ok(blob) => Ok(blob),
        Err(error) => Err(error.clone()),
    }
}

#[cfg(test)]
pub(crate) fn module_cache_len() -> usize {
    MODULES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("module cache mutex should not be poisoned")
        .len()
}

#[cfg(test)]
pub(crate) fn instance_pre_cache_len() -> usize {
    INSTANCE_PRES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("instance-pre cache mutex should not be poisoned")
        .len()
}

#[cfg(test)]
pub(crate) fn execution_instance_pool_len(aux_size: u32) -> usize {
    EXECUTION_INSTANCES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("execution-instance pool mutex should not be poisoned")
        .get(&aux_size)
        .map(Vec::len)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        cached_execution_instance, cached_instance_pre, cached_module, execution_instance_pool_len,
        instance_pre_cache_len, module_cache_len,
    };
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn cache_test_guard() -> MutexGuard<'static, ()> {
        static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("cache test mutex should not be poisoned")
    }

    #[test]
    fn reuses_cached_module_for_same_aux_size() {
        let _guard = cache_test_guard();
        let aux_a = 0x00f0_0001;
        let aux_b = 0x00f0_0002;
        let start = module_cache_len();

        cached_module(aux_a).expect("first module should compile");
        let after_first = module_cache_len();
        assert_eq!(after_first, start + 1);

        cached_module(aux_a).expect("same aux size should reuse cache");
        assert_eq!(module_cache_len(), after_first);

        cached_module(aux_b).expect("different aux size should allocate one new entry");
        assert_eq!(module_cache_len(), after_first + 1);
    }

    #[test]
    fn reuses_cached_instance_pre_for_same_aux_size() {
        let _guard = cache_test_guard();
        let aux_a = 0x00f0_1001;
        let aux_b = 0x00f0_1002;
        let start = instance_pre_cache_len();

        cached_instance_pre(aux_a).expect("first instance pre should compile");
        let after_first = instance_pre_cache_len();
        assert_eq!(after_first, start + 1);

        cached_instance_pre(aux_a).expect("same aux size should reuse cached instance pre");
        assert_eq!(instance_pre_cache_len(), after_first);

        cached_instance_pre(aux_b)
            .expect("different aux size should allocate one new instance pre");
        assert_eq!(instance_pre_cache_len(), after_first + 1);
    }

    #[test]
    fn returns_execution_instances_to_pool() {
        let _guard = cache_test_guard();
        let aux_size = 0x00f0_2001;
        let start = execution_instance_pool_len(aux_size);

        {
            let _instance =
                cached_execution_instance(aux_size).expect("pooled instance should be available");
        }

        assert_eq!(execution_instance_pool_len(aux_size), start + 1);
    }
}
