# Neo N3 Native Contracts Support

⚠️ **ARCHITECTURE NOTE**: Native contract semantics remain defined and executed by the Neo N3 C# engine. The Rust devpack provides thin wrappers that route through `System.Contract.Call` via the guest/host FFI bridge; it does **not** reimplement native contract logic in Rust.

## Coverage

Neo N3 has 11 native contracts. The devpack intends to provide one Rust module per contract under `neo_riscv_devpack::native::*`.

| Native contract | Rust module | Status |
| --- | --- | --- |
| ContractManagement | `native::contract_management` | Exposed |
| StdLib | `native::std_lib` | Exposed |
| CryptoLib | `native::crypto_lib` | Exposed |
| LedgerContract | `native::ledger` | Exposed |
| NeoToken | `native::neo_token` | Exposed |
| GasToken | `native::gas_token` | Exposed |
| PolicyContract | `native::policy` | Exposed |
| RoleManagement | `native::role_management` | Exposed |
| OracleContract | `native::oracle` | Exposed |
| Notary | `native::notary` | Exposed |
| Treasury | `native::treasury` | Exposed |

## Rust Surface (Current)

The following lists the current Rust-facing names in `crates/neo-riscv-devpack/src/native/*`. These bindings call native contracts through `System.Contract.Call` using the canonical Neo native hashes.

### NeoToken

- `NEO_TOKEN_HASH`
- `neo_balance_of`
- `neo_transfer`
- `neo_get_candidates`
- `neo_register_candidate`
- `neo_vote`
- `neo_unclaimed_gas`
- `neo_symbol`
- `neo_decimals`
- `neo_total_supply`

### GasToken

- `GAS_TOKEN_HASH`
- `gas_balance_of`
- `gas_transfer`
- `gas_symbol`
- `gas_decimals`
- `gas_total_supply`

### PolicyContract

- `policy_get_fee_per_byte`
- `policy_get_exec_fee_factor`
- `policy_get_storage_price`
- `policy_is_blocked`
- `policy_get_attribute_fee`
- `policy_get_milliseconds_per_block`
- `policy_get_max_valid_until_block_increment`
- `policy_get_max_traceable_blocks`

### ContractManagement

- `contract_deploy`
- `contract_update`
- `contract_destroy`
- `contract_get_contract`

### CryptoLib

- `crypto_sha256`
- `crypto_ripemd160`
- `crypto_verify_with_ecdsa`
- `crypto_murmur32`
- `crypto_keccak256`
- `crypto_verify_with_ed25519`

### StdLib

- `stdlib_serialize`
- `stdlib_deserialize`
- `stdlib_json_serialize`
- `stdlib_json_deserialize`
- `stdlib_base64_encode`
- `stdlib_base64_decode`
- `stdlib_itoa`
- `stdlib_atoi`
- `stdlib_base58_encode`
- `stdlib_base58_decode`

### Oracle

- `oracle_request`

### RoleManagement

- `role_get_designated_by_role`

### Ledger

- `ledger_get_block`
- `ledger_get_transaction`
- `ledger_current_index`
- `ledger_get_transaction_height`
- `ledger_current_hash`

### Notary

- `NOTARY_HASH`
- `notary_balance_of`
- `notary_expiration_of`
- `notary_get_max_not_valid_before_delta`
- `notary_lock_deposit_until`
- `notary_withdraw`
- `notary_verify`
- `notary_set_max_not_valid_before_delta`

### Treasury

- `TREASURY_HASH`
- `treasury_verify`
- `treasury_on_nep17_payment`
- `treasury_on_nep11_payment`
