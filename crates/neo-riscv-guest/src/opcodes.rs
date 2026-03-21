// Many constants are used via range patterns (e.g., LDLOC0..=LDLOC6) and
// appear "unused" to the compiler. Suppress the warning at module scope.
#![allow(dead_code)]

// Push
pub(crate) const PUSHINT8: u8 = 0x00;
pub(crate) const PUSHINT16: u8 = 0x01;
pub(crate) const PUSHINT32: u8 = 0x02;
pub(crate) const PUSHINT64: u8 = 0x03;
pub(crate) const PUSHINT128: u8 = 0x04;
pub(crate) const PUSHINT256: u8 = 0x05;
pub(crate) const PUSHT: u8 = 0x08;
pub(crate) const PUSHF: u8 = 0x09;
pub(crate) const PUSHA: u8 = 0x0a;
pub(crate) const PUSHNULL: u8 = 0x0b;
pub(crate) const PUSHDATA1: u8 = 0x0c;
pub(crate) const PUSHDATA2: u8 = 0x0d;
pub(crate) const PUSHDATA4: u8 = 0x0e;
pub(crate) const PUSHM1: u8 = 0x0f;
pub(crate) const PUSH0: u8 = 0x10;
pub(crate) const PUSH1: u8 = 0x11;
pub(crate) const PUSH2: u8 = 0x12;
pub(crate) const PUSH3: u8 = 0x13;
pub(crate) const PUSH4: u8 = 0x14;
pub(crate) const PUSH5: u8 = 0x15;
pub(crate) const PUSH6: u8 = 0x16;
pub(crate) const PUSH7: u8 = 0x17;
pub(crate) const PUSH8: u8 = 0x18;
pub(crate) const PUSH9: u8 = 0x19;
pub(crate) const PUSH10: u8 = 0x1a;
pub(crate) const PUSH11: u8 = 0x1b;
pub(crate) const PUSH12: u8 = 0x1c;
pub(crate) const PUSH13: u8 = 0x1d;
pub(crate) const PUSH14: u8 = 0x1e;
pub(crate) const PUSH15: u8 = 0x1f;
pub(crate) const PUSH16: u8 = 0x20;

// Flow control
pub(crate) const NOP: u8 = 0x21;
pub(crate) const JMP: u8 = 0x22;
pub(crate) const JMP_L: u8 = 0x23;
pub(crate) const JMPIF: u8 = 0x24;
pub(crate) const JMPIF_L: u8 = 0x25;
pub(crate) const JMPIFNOT: u8 = 0x26;
pub(crate) const JMPIFNOT_L: u8 = 0x27;
pub(crate) const JMPEQ: u8 = 0x28;
pub(crate) const JMPEQ_L: u8 = 0x29;
pub(crate) const JMPNE: u8 = 0x2a;
pub(crate) const JMPNE_L: u8 = 0x2b;
pub(crate) const JMPGT: u8 = 0x2c;
pub(crate) const JMPGT_L: u8 = 0x2d;
pub(crate) const JMPGE: u8 = 0x2e;
pub(crate) const JMPGE_L: u8 = 0x2f;
pub(crate) const JMPLT: u8 = 0x30;
pub(crate) const JMPLT_L: u8 = 0x31;
pub(crate) const JMPLE: u8 = 0x32;
pub(crate) const JMPLE_L: u8 = 0x33;
pub(crate) const CALL: u8 = 0x34;
pub(crate) const CALL_L: u8 = 0x35;
pub(crate) const CALLA: u8 = 0x36;
pub(crate) const ABORT: u8 = 0x38;
pub(crate) const ASSERT: u8 = 0x39;
pub(crate) const ABORTMSG: u8 = 0xe0;
pub(crate) const ASSERTMSG: u8 = 0xe1;
pub(crate) const RET: u8 = 0x40;
pub(crate) const SYSCALL: u8 = 0x41;

// Stack
pub(crate) const TOALTSTACK: u8 = 0x06;
pub(crate) const FROMALTSTACK: u8 = 0x07;
pub(crate) const DEPTH: u8 = 0x43;
pub(crate) const DROP: u8 = 0x45;
pub(crate) const DUP: u8 = 0x4a;
pub(crate) const SWAP: u8 = 0x50;
pub(crate) const CLEAR: u8 = 0x49;
pub(crate) const OVER: u8 = 0x4b;
pub(crate) const PICK: u8 = 0x4d;
pub(crate) const ROT: u8 = 0x51;
pub(crate) const ROLL: u8 = 0x52;
pub(crate) const REVERSE3: u8 = 0x53;
pub(crate) const REVERSE4: u8 = 0x54;
pub(crate) const REVERSEN: u8 = 0x55;
pub(crate) const NIP: u8 = 0x46;
pub(crate) const TUCK: u8 = 0x4e;
pub(crate) const XDROP: u8 = 0x48;

// Slot
pub(crate) const INITSSLOT: u8 = 0x56;
pub(crate) const INITSLOT: u8 = 0x57;
pub(crate) const LDSFLD0: u8 = 0x58;
pub(crate) const LDSFLD1: u8 = 0x59;
pub(crate) const LDSFLD2: u8 = 0x5a;
pub(crate) const LDSFLD3: u8 = 0x5b;
pub(crate) const LDSFLD4: u8 = 0x5c;
pub(crate) const LDSFLD5: u8 = 0x5d;
pub(crate) const LDSFLD6: u8 = 0x5e;
pub(crate) const LDSFLD: u8 = 0x5f;
pub(crate) const STSFLD0: u8 = 0x60;
pub(crate) const STSFLD1: u8 = 0x61;
pub(crate) const STSFLD2: u8 = 0x62;
pub(crate) const STSFLD3: u8 = 0x63;
pub(crate) const STSFLD4: u8 = 0x64;
pub(crate) const STSFLD5: u8 = 0x65;
pub(crate) const STSFLD6: u8 = 0x66;
pub(crate) const STSFLD: u8 = 0x67;
pub(crate) const LDLOC0: u8 = 0x68;
pub(crate) const LDLOC1: u8 = 0x69;
pub(crate) const LDLOC2: u8 = 0x6a;
pub(crate) const LDLOC3: u8 = 0x6b;
pub(crate) const LDLOC4: u8 = 0x6c;
pub(crate) const LDLOC5: u8 = 0x6d;
pub(crate) const LDLOC6: u8 = 0x6e;
pub(crate) const LDLOC: u8 = 0x6f;
pub(crate) const STLOC0: u8 = 0x70;
pub(crate) const STLOC1: u8 = 0x71;
pub(crate) const STLOC2: u8 = 0x72;
pub(crate) const STLOC3: u8 = 0x73;
pub(crate) const STLOC4: u8 = 0x74;
pub(crate) const STLOC5: u8 = 0x75;
pub(crate) const STLOC6: u8 = 0x76;
pub(crate) const STLOC: u8 = 0x77;
pub(crate) const LDARG0: u8 = 0x78;
pub(crate) const LDARG1: u8 = 0x79;
pub(crate) const LDARG2: u8 = 0x7a;
pub(crate) const LDARG3: u8 = 0x7b;
pub(crate) const LDARG4: u8 = 0x7c;
pub(crate) const LDARG5: u8 = 0x7d;
pub(crate) const LDARG6: u8 = 0x7e;
pub(crate) const LDARG: u8 = 0x7f;
pub(crate) const STARG0: u8 = 0x80;
pub(crate) const STARG1: u8 = 0x81;
pub(crate) const STARG2: u8 = 0x82;
pub(crate) const STARG3: u8 = 0x83;
pub(crate) const STARG4: u8 = 0x84;
pub(crate) const STARG5: u8 = 0x85;
pub(crate) const STARG6: u8 = 0x86;
pub(crate) const STARG: u8 = 0x87;

// Splice
pub(crate) const NEWBUFFER: u8 = 0x88;
pub(crate) const MEMCPY: u8 = 0x89;
pub(crate) const CAT: u8 = 0x8b;
pub(crate) const SUBSTR: u8 = 0x8c;
pub(crate) const LEFT: u8 = 0x8d;
pub(crate) const RIGHT: u8 = 0x8e;

// Bitwise logic
pub(crate) const INVERT: u8 = 0x90;
pub(crate) const AND: u8 = 0x91;
pub(crate) const OR: u8 = 0x92;
pub(crate) const XOR: u8 = 0x93;
pub(crate) const EQUAL: u8 = 0x97;
pub(crate) const NOTEQUAL: u8 = 0x98;

// Arithmetic
pub(crate) const SIGN: u8 = 0x99;
pub(crate) const ABS: u8 = 0x9a;
pub(crate) const NEGATE: u8 = 0x9b;
pub(crate) const INC: u8 = 0x9c;
pub(crate) const DEC: u8 = 0x9d;
pub(crate) const ADD: u8 = 0x9e;
pub(crate) const SUB: u8 = 0x9f;
pub(crate) const MUL: u8 = 0xa0;
pub(crate) const DIV: u8 = 0xa1;
pub(crate) const MOD: u8 = 0xa2;
pub(crate) const POW: u8 = 0xa3;
pub(crate) const SQRT: u8 = 0xa4;
pub(crate) const MODMUL: u8 = 0xa5;
pub(crate) const MODPOW: u8 = 0xa6;
pub(crate) const SHL: u8 = 0xa8;
pub(crate) const SHR: u8 = 0xa9;
pub(crate) const NOT: u8 = 0xaa;
pub(crate) const BOOLAND: u8 = 0xab;
pub(crate) const BOOLOR: u8 = 0xac;
pub(crate) const NZ: u8 = 0xb1;
pub(crate) const NUMEQUAL: u8 = 0xb3;
pub(crate) const NUMNOTEQUAL: u8 = 0xb4;
pub(crate) const LT: u8 = 0xb5;
pub(crate) const LE: u8 = 0xb6;
pub(crate) const GT: u8 = 0xb7;
pub(crate) const GE: u8 = 0xb8;
pub(crate) const MIN: u8 = 0xb9;
pub(crate) const MAX: u8 = 0xba;
pub(crate) const WITHIN: u8 = 0xbb;

// Compound types
pub(crate) const PACKMAP: u8 = 0xbe;
pub(crate) const PACKSTRUCT: u8 = 0xbf;
pub(crate) const PACK: u8 = 0xc0;
pub(crate) const UNPACK: u8 = 0xc1;
pub(crate) const NEWARRAY0: u8 = 0xc2;
pub(crate) const NEWARRAY: u8 = 0xc3;
pub(crate) const NEWARRAY_T: u8 = 0xc4;
pub(crate) const NEWSTRUCT0: u8 = 0xc5;
pub(crate) const NEWSTRUCT: u8 = 0xc6;
pub(crate) const NEWMAP: u8 = 0xc8;
pub(crate) const SIZE: u8 = 0xca;
pub(crate) const HASKEY: u8 = 0xcb;
pub(crate) const KEYS: u8 = 0xcc;
pub(crate) const VALUES: u8 = 0xcd;
pub(crate) const PICKITEM: u8 = 0xce;
pub(crate) const APPEND: u8 = 0xcf;
pub(crate) const SETITEM: u8 = 0xd0;
pub(crate) const REVERSEITEMS: u8 = 0xd1;
pub(crate) const REMOVE: u8 = 0xd2;
pub(crate) const CLEARITEMS: u8 = 0xd3;
pub(crate) const POPITEM: u8 = 0xd4;

// Calls
pub(crate) const CALLT: u8 = 0x37;

// Exceptions
pub(crate) const THROW: u8 = 0x3a;
pub(crate) const THROWIFNOT: u8 = 0xf1;

// Exception handling
pub(crate) const TRY: u8 = 0x3b;
pub(crate) const TRY_L: u8 = 0x3c;
pub(crate) const ENDTRY: u8 = 0x3d;
pub(crate) const ENDTRY_L: u8 = 0x3e;
pub(crate) const ENDFINALLY: u8 = 0x3f;

// Types
pub(crate) const ISTYPE: u8 = 0xd9;
pub(crate) const ISNULL: u8 = 0xd8;
pub(crate) const CONVERT: u8 = 0xdb;
