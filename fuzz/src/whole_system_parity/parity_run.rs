use neo_riscv_abi::ExecutionResult;

use super::TraceEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParityRun {
    pub(crate) outcome: Result<ExecutionResult, String>,
    pub(crate) trace: Vec<TraceEntry>,
}
