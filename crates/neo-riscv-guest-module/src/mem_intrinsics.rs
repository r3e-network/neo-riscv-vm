//! Minimal C memory intrinsics for the no_std PolkaVM guest module.
//!
//! The riscv32 PolkaVM target can emit calls to these symbols, while
//! `compiler_builtins` does not export them as C ABI functions.
//!
//! These are written as explicit element loops rather than delegating to
//! `core::ptr::{copy, copy_nonoverlapping, write_bytes}`. Those helpers lower
//! to `memcpy`/`memset`/`memmove` libcalls for runtime-sized lengths, and
//! because this module *defines* those very symbols the call would recurse
//! into itself and overflow the guest stack. An explicit loop in the body of
//! the `memcpy` definition is not re-synthesized by LLVM into a self-call on
//! this target, so the loop form is the safe one.
//!
//! Unsafe pointer operations are wrapped in explicit `unsafe {}` blocks
//! (required under edition 2024's `unsafe_op_in_unsafe_fn`).

/// Copy `n` bytes from `src` to `dest` (non-overlapping). C `memcpy`.
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        // SAFETY: caller guarantees [src,src+n) and [dest,dest+n) are valid
        // and non-overlapping (C memcpy contract).
        unsafe { dest.add(i).write(src.add(i).read()) };
        i += 1;
    }
    dest
}

/// Fill `n` bytes at `s` with byte value `c`. C `memset`.
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        // SAFETY: caller guarantees [s,s+n) is valid for writes.
        unsafe { s.add(i).write(c as u8) };
        i += 1;
    }
    s
}

/// Copy `n` bytes from `src` to `dest` (may overlap). C `memmove`.
///
/// Copies back-to-front when the regions overlap with `src < dest`, otherwise
/// front-to-back, so overlapping ranges are handled correctly.
#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (src as usize) < (dest as usize) {
        let mut i = n;
        while i > 0 {
            i -= 1;
            // SAFETY: caller guarantees both ranges are valid for n bytes.
            unsafe { dest.add(i).write(src.add(i).read()) };
        }
    } else {
        let mut i = 0;
        while i < n {
            // SAFETY: caller guarantees both ranges are valid for n bytes.
            unsafe { dest.add(i).write(src.add(i).read()) };
            i += 1;
        }
    }
    dest
}

/// Compare `n` bytes at `s1` and `s2`. C `memcmp`.
///
/// Returns the signed difference of the first differing byte pair, or 0 if all
/// `n` bytes match.
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        // SAFETY: caller guarantees both ranges are valid for reads of n bytes.
        let a = unsafe { s1.add(i).read() };
        let b = unsafe { s2.add(i).read() };
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}
