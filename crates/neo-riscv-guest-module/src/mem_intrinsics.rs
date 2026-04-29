//! Minimal C memory intrinsics for the no_std PolkaVM guest module.
//!
//! The riscv32 PolkaVM target can emit calls to these symbols, while
//! `compiler_builtins` does not export them as C ABI functions.

#[no_mangle]
unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        dest.add(i).write(src.add(i).read());
        i += 1;
    }
    dest
}

#[no_mangle]
unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        s.add(i).write(c as u8);
        i += 1;
    }
    s
}

#[no_mangle]
unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest as *const u8 {
        let mut i = n;
        while i > 0 {
            i -= 1;
            dest.add(i).write(src.add(i).read());
        }
    } else {
        let mut i = 0;
        while i < n {
            dest.add(i).write(src.add(i).read());
            i += 1;
        }
    }
    dest
}

#[no_mangle]
unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = s1.add(i).read();
        let b = s2.add(i).read();
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}
