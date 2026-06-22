//! Minimal C memory intrinsics for the no_std PolkaVM guest module.
//!
//! The riscv32 PolkaVM target can emit calls to these symbols, while
//! `compiler_builtins` does not export them as C ABI functions.
//!
//! These implementations delegate to [`core::ptr`] primitives
//! (`copy_nonoverlapping`, `copy`, `write_bytes`) so LLVM can lower them to
//! word-stride loops instead of the byte-at-a-time loops a naive translation
//! would produce — important because the PolkaVM guest counts instructions.

use core::ptr;

/// Copy `n` bytes from `src` to `dest` (non-overlapping).
///
/// Equivalent to C `memcpy`. Uses [`ptr::copy_nonoverlapping`].
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // SAFETY: caller guarantees [src, src+n) and [dest, dest+n) are valid and
    // non-overlapping (C memcpy contract).
    unsafe { ptr::copy_nonoverlapping(src, dest, n) };
    dest
}

/// Fill `n` bytes at `s` with byte value `c`.
///
/// Equivalent to C `memset`. Uses [`ptr::write_bytes`].
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    // SAFETY: caller guarantees [s, s+n) is valid for writes.
    unsafe { ptr::write_bytes(s, c as u8, n) };
    s
}

/// Copy `n` bytes from `src` to `dest` (may overlap).
///
/// Equivalent to C `memmove`. Uses [`ptr::copy`], which handles overlapping
/// regions correctly (LLVM picks the direction).
#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // SAFETY: caller guarantees both ranges are valid; ptr::copy handles overlap.
    unsafe { ptr::copy(src, dest, n) };
    dest
}

/// Compare `n` bytes at `s1` and `s2`.
///
/// Equivalent to C `memcmp`. Returns the difference of the first differing
/// pair as `i32`, or 0 if all bytes match. Kept as a byte loop because the
/// early-return-on-mismatch semantics don't benefit from bulk copy.
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        // SAFETY: caller guarantees both ranges valid for reads of n bytes.
        let a = unsafe { s1.add(i).read() };
        let b = unsafe { s2.add(i).read() };
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}
