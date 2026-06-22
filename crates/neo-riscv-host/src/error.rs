use thiserror::Error;

/// Typed error for the Neo RISC-V host runtime.
///
/// Replaces pervasive `Result<_, String>` throughout the crate with
/// structured error variants that support `Display`, `Error`, and
/// `#[from]` delegation for the underlying libraries.
#[derive(Debug, Error, Clone)]
pub enum HostError {
    /// The guest (NeoVM compatibility interpreter or native contract) reported a
    /// generic execution failure with a human-readable message.
    #[error("guest execution failed: {0}")]
    GuestExecution(String),

    /// Execution exceeded the gas limit (`consumed` vs `limit`).
    #[error("gas exhausted: consumed {consumed}, limit {limit}")]
    GasExhausted {
        /// Gas units actually consumed before the limit was hit.
        consumed: u64,
        /// The gas limit that was exceeded.
        limit: u64,
    },

    /// Execution hit the NeoVM instruction-count ceiling (`count` vs `ceiling`).
    #[error("instruction ceiling reached: {count}/{ceiling}")]
    InstructionCeiling {
        /// Number of NeoVM instructions executed before the ceiling was hit.
        count: u64,
        /// The instruction-count ceiling that was reached.
        ceiling: u64,
    },

    /// A native RISC-V contract call returned an error.
    #[error("native contract execution failed: {0}")]
    NativeContractError(String),

    /// The PolkaVM engine itself raised an error (compilation, linking, etc.).
    #[error("PolkaVM error: {0}")]
    PolkaVm(String),

    /// The result payload returned by the guest could not be decoded.
    #[error("failed to decode result: {0}")]
    DecodeError(String),

    /// A memory allocation or arena operation failed.
    #[error("memory error: {0}")]
    MemoryError(String),

    /// A codec/serialization step failed.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// The built-in host callback was invoked in a release build where it is
    /// intentionally disabled.
    #[error("builtin host callback is disabled in release builds")]
    BuiltinDisabled,

    /// Catch-all for errors not covered by a more specific variant.
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
