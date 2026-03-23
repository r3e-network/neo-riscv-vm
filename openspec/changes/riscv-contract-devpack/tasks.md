## 1. RISC-V Compiler Setup

- [x] 1.1 Configure Rust toolchain with riscv32emac target
- [x] 1.2 Create cargo-riscv build wrapper
- [x] 1.3 Implement PolkaVM binary output format
- [x] 1.4 Add no_std compilation support
- [x] 1.5 Verify binary compatibility with neo-riscv-vm

## 2. Contract Devpack Implementation

- [x] 2.1 Create neo-riscv-devpack crate structure
- [x] 2.2 Implement Neo N3 syscall bindings (System.Contract.Call, etc.)
- [x] 2.3 Add standard types (Hash160, Hash256, PublicKey)
- [x] 2.4 Create contract attribute macros
- [x] 2.5 Add storage helper functions
- [x] 2.6 Implement event emission utilities

## 3. Contract Standards

- [x] 3.1 Define manifest schema (JSON format)
- [x] 3.2 Implement manifest generator from Rust attributes
- [x] 3.3 Define standard entry point convention
- [x] 3.4 Create ABI descriptor format
- [x] 3.5 Add permission and trust declarations

## 4. Deployment Tools

- [x] 4.1 Create contract packaging tool (binary + manifest)
- [x] 4.2 Implement deployment transaction builder
- [x] 4.3 Add network configuration (mainnet/testnet)
- [x] 4.4 Create deployment CLI
- [x] 4.5 Add contract verification utilities

## 5. Invocation Utilities

- [x] 5.1 Implement contract invocation builder
- [x] 5.2 Add parameter encoding/decoding
- [x] 5.3 Create storage query utilities
- [x] 5.4 Add transaction signing support
- [x] 5.5 Implement result parsing

## 6. Documentation & Examples

- [x] 6.1 Write getting started guide
- [x] 6.2 Create hello-world contract example
- [x] 6.3 Add NEP-17 token example
- [x] 6.4 Document syscall API reference
- [x] 6.5 Add deployment workflow guide
