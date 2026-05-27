use polkavm::Module;

use super::{
    native_cache_key::NativeCacheKey, CachedInstancePre, ExecutionInstance, MAX_POOL_SIZE_PER_AUX,
    NATIVE_EXECUTION_INSTANCES,
};

pub(crate) struct CachedNativeExecutionInstance {
    pub(super) key: NativeCacheKey,
    pub(super) instance_pre: CachedInstancePre,
    pub(super) instance: Option<ExecutionInstance>,
}

impl CachedNativeExecutionInstance {
    pub(crate) fn module(&self) -> &Module {
        self.instance_pre.module()
    }

    pub(crate) fn instance_mut(&mut self) -> &mut ExecutionInstance {
        self.instance
            .as_mut()
            .expect("cached native execution instance should be present")
    }
}

impl Drop for CachedNativeExecutionInstance {
    fn drop(&mut self) {
        let Some(instance) = self.instance.take() else {
            return;
        };

        if let Some(pool) = NATIVE_EXECUTION_INSTANCES.get()
            && let Ok(mut guard) = pool.lock() {
                let instances = guard.entry(self.key).or_default();
                if instances.len() < MAX_POOL_SIZE_PER_AUX {
                    instances.push(instance);
                }
            }
    }
}
