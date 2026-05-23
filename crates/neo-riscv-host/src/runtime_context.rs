/// Runtime execution context for VM scripts.
#[derive(Clone, Copy)]
pub struct RuntimeContext {
    /// Trigger type (Application, Verification, etc.).
    pub trigger: u8,
    /// Network magic number.
    pub network: u32,
    /// Address version byte.
    pub address_version: u8,
    /// Block timestamp (optional).
    pub timestamp: Option<u64>,
    /// Remaining gas.
    pub gas_left: i64,
    /// Gas price factor in pico units.
    pub exec_fee_factor_pico: i64,
}
