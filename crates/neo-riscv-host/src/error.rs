use thiserror::Error;

/// Typed error for the Neo RISC-V host runtime.
///
/// Replaces pervasive `Result<_, String>` throughout the crate with
/// structured error variants that support `Display`, `Error`, and
/// `#[from]` delegation for the underlying libraries.
#[derive(Debug, Error, Clone)]
pub enum HostError {
    #[error("guest execution failed: {0}")]
    GuestExecution(String),

    #[error("gas exhausted: consumed {consumed}, limit {limit}")]
    GasExhausted { consumed: u64, limit: u64 },

    #[error("instruction ceiling reached: {count}/{ceiling}")]
    InstructionCeiling { count: u64, ceiling: u64 },

    #[error("native contract execution failed: {0}")]
    NativeContractError(String),

    #[error("PolkaVM error: {0}")]
    PolkaVm(String),

    #[error("failed to decode result: {0}")]
    DecodeError(String),

    #[error("memory error: {0}")]
    MemoryError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("builtin host callback is disabled in release builds")]
    BuiltinDisabled,

    #[error("{0}")]
    Other(String),
}

impl From<String> for HostError {
    fn from(s: String) -> Self {
        HostError::Other(s)
    }
}

impl From<&str> for HostError {
    fn from(s: &str) -> Self {
        HostError::Other(s.to_string())
    }
}

impl From<HostError> for String {
    fn from(e: HostError) -> Self {
        e.to_string()
    }
}
