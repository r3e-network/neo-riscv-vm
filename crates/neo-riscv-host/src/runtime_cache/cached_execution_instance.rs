use polkavm::Module;

use super::{CachedInstancePre, ExecutionInstance, EXECUTION_INSTANCES, MAX_POOL_SIZE_PER_AUX};

pub(crate) struct CachedExecutionInstance {
    pub(super) aux_size: u32,
    pub(super) instance_pre: CachedInstancePre,
    pub(super) instance: Option<ExecutionInstance>,
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
                let instances = guard.entry(self.aux_size).or_default();
                if instances.len() < MAX_POOL_SIZE_PER_AUX {
                    instances.push(instance);
                }
                // else: pool is full, just drop the instance
            }
        }
    }
}
