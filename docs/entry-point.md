# Contract Entry Point Convention

## Standard Entry Point

All RISC-V contracts MUST export a function:

```rust
#[no_mangle]
pub extern "C" fn invoke(method: *const u8, args: *const u8) -> i32 {
    // Contract logic
    0
}
```

## Return Values

- 0: Success
- Non-zero: Error code
