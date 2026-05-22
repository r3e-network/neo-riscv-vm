use crate::RuntimeContext;
use neo_riscv_abi::OpCode;

/// Hard instruction-count ceiling for a single `execute_script` call. Guards the
/// block processor against infinite-loop bugs in the NeoVM-on-RISC-V guest that
/// don't exhaust gas fast enough (e.g. cheap opcodes in a tight loop, or
/// `exec_fee_factor_pico == 0`). A real transaction should not need this many
/// NeoVM opcodes — hitting this cap FAULTs with a bounded cost instead of
/// hanging the block indefinitely.
///
/// Known mainnet triggers: blocks 480,305 / 510,447 / 510,448 previously required
/// `--skip-block` workarounds; this watchdog converts those hangs to terminating
/// FAULTs so the validator / node can make forward progress.
pub(crate) const NEO_INSTRUCTION_CEILING: u64 = 1_000_000_000;

/// Return `Err` with a descriptive message if the running opcode count has reached
/// the instruction ceiling. `count` is the already-incremented count for the
/// current opcode — i.e. callers should post-increment then call this.
pub(crate) fn check_instruction_ceiling(count: u64) -> Result<(), String> {
    if count >= NEO_INSTRUCTION_CEILING {
        Err(format!(
            "execution exceeded instruction ceiling {NEO_INSTRUCTION_CEILING} (count={count})"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn charge_opcode(
    context: &mut RuntimeContext,
    fee_consumed_pico: &mut i64,
    opcode: u8,
) -> Result<(), String> {
    if context.exec_fee_factor_pico == 0 {
        return Ok(());
    }

    let delta = opcode_price(opcode)
        .checked_mul(context.exec_fee_factor_pico)
        .ok_or_else(|| "opcode fee overflow".to_string())?;
    let previous_datoshi = fee_consumed_pico.saturating_add(9_999) / 10_000;
    *fee_consumed_pico = fee_consumed_pico
        .checked_add(delta)
        .ok_or_else(|| "opcode fee overflow".to_string())?;
    let current_datoshi = fee_consumed_pico.saturating_add(9_999) / 10_000;
    let consumed_delta = current_datoshi.saturating_sub(previous_datoshi);

    context.gas_left = context
        .gas_left
        .checked_sub(consumed_delta)
        .ok_or_else(|| "Insufficient GAS.".to_string())?;
    if context.gas_left < 0 {
        return Err("Insufficient GAS.".to_string());
    }
    Ok(())
}

pub(crate) fn native_instruction_limit(context: &RuntimeContext) -> i64 {
    if context.exec_fee_factor_pico <= 0 {
        return NEO_INSTRUCTION_CEILING as i64;
    }

    if context.gas_left <= 0 {
        return 0;
    }

    let available_pico = i128::from(context.gas_left) * 10_000;
    let units = available_pico / i128::from(context.exec_fee_factor_pico);
    units
        .min(i128::from(NEO_INSTRUCTION_CEILING))
        .min(i128::from(i64::MAX))
        .max(0) as i64
}

pub(crate) fn charge_native_instructions(
    context: &mut RuntimeContext,
    fee_consumed_pico: &mut i64,
    instruction_count: i64,
) -> Result<(), String> {
    if instruction_count <= 0 || context.exec_fee_factor_pico == 0 {
        return Ok(());
    }

    if context.exec_fee_factor_pico < 0 {
        return Err("negative execution fee factor".to_string());
    }

    let delta = instruction_count
        .checked_mul(context.exec_fee_factor_pico)
        .ok_or_else(|| "native instruction fee overflow".to_string())?;
    let previous_datoshi = fee_consumed_pico.saturating_add(9_999) / 10_000;
    *fee_consumed_pico = fee_consumed_pico
        .checked_add(delta)
        .ok_or_else(|| "native instruction fee overflow".to_string())?;
    let current_datoshi = fee_consumed_pico.saturating_add(9_999) / 10_000;
    let consumed_delta = current_datoshi.saturating_sub(previous_datoshi);

    context.gas_left = context
        .gas_left
        .checked_sub(consumed_delta)
        .ok_or_else(|| "Insufficient GAS.".to_string())?;
    if context.gas_left < 0 {
        return Err("Insufficient GAS.".to_string());
    }
    Ok(())
}

pub(crate) fn opcode_price(opcode: u8) -> i64 {
    OpCode::try_from(opcode).map_or(65_536, opcode_price_for)
}

fn opcode_price_for(opcode: OpCode) -> i64 {
    match opcode {
        OpCode::ABORT | OpCode::RET | OpCode::SYSCALL | OpCode::ABORTMSG => 0,
        OpCode::PUSHINT8
        | OpCode::PUSHINT16
        | OpCode::PUSHINT32
        | OpCode::PUSHINT64
        | OpCode::PUSHT
        | OpCode::PUSHF
        | OpCode::PUSHNULL
        | OpCode::PUSHM1
        | OpCode::PUSH0
        | OpCode::PUSH1
        | OpCode::PUSH2
        | OpCode::PUSH3
        | OpCode::PUSH4
        | OpCode::PUSH5
        | OpCode::PUSH6
        | OpCode::PUSH7
        | OpCode::PUSH8
        | OpCode::PUSH9
        | OpCode::PUSH10
        | OpCode::PUSH11
        | OpCode::PUSH12
        | OpCode::PUSH13
        | OpCode::PUSH14
        | OpCode::PUSH15
        | OpCode::PUSH16
        | OpCode::NOP
        | OpCode::ASSERT
        | OpCode::ASSERTMSG => 1,
        OpCode::JMP
        | OpCode::JMP_L
        | OpCode::JMPIF
        | OpCode::JMPIF_L
        | OpCode::JMPIFNOT
        | OpCode::JMPIFNOT_L
        | OpCode::JMPEQ
        | OpCode::JMPEQ_L
        | OpCode::JMPNE
        | OpCode::JMPNE_L
        | OpCode::JMPGT
        | OpCode::JMPGT_L
        | OpCode::JMPGE
        | OpCode::JMPGE_L
        | OpCode::JMPLT
        | OpCode::JMPLT_L
        | OpCode::JMPLE
        | OpCode::JMPLE_L
        | OpCode::DEPTH
        | OpCode::DROP
        | OpCode::NIP
        | OpCode::DUP
        | OpCode::OVER
        | OpCode::PICK
        | OpCode::TUCK
        | OpCode::SWAP
        | OpCode::ROT
        | OpCode::REVERSE3
        | OpCode::REVERSE4
        | OpCode::LDSFLD0
        | OpCode::LDSFLD1
        | OpCode::LDSFLD2
        | OpCode::LDSFLD3
        | OpCode::LDSFLD4
        | OpCode::LDSFLD5
        | OpCode::LDSFLD6
        | OpCode::LDSFLD
        | OpCode::STSFLD0
        | OpCode::STSFLD1
        | OpCode::STSFLD2
        | OpCode::STSFLD3
        | OpCode::STSFLD4
        | OpCode::STSFLD5
        | OpCode::STSFLD6
        | OpCode::STSFLD
        | OpCode::LDLOC0
        | OpCode::LDLOC1
        | OpCode::LDLOC2
        | OpCode::LDLOC3
        | OpCode::LDLOC4
        | OpCode::LDLOC5
        | OpCode::LDLOC6
        | OpCode::LDLOC
        | OpCode::STLOC0
        | OpCode::STLOC1
        | OpCode::STLOC2
        | OpCode::STLOC3
        | OpCode::STLOC4
        | OpCode::STLOC5
        | OpCode::STLOC6
        | OpCode::STLOC
        | OpCode::LDARG0
        | OpCode::LDARG1
        | OpCode::LDARG2
        | OpCode::LDARG3
        | OpCode::LDARG4
        | OpCode::LDARG5
        | OpCode::LDARG6
        | OpCode::LDARG
        | OpCode::STARG0
        | OpCode::STARG1
        | OpCode::STARG2
        | OpCode::STARG3
        | OpCode::STARG4
        | OpCode::STARG5
        | OpCode::STARG6
        | OpCode::STARG
        | OpCode::ISNULL
        | OpCode::ISTYPE => 2,
        OpCode::PUSHINT128
        | OpCode::PUSHINT256
        | OpCode::PUSHA
        | OpCode::TRY
        | OpCode::TRY_L
        | OpCode::ENDTRY
        | OpCode::ENDTRY_L
        | OpCode::ENDFINALLY
        | OpCode::INVERT
        | OpCode::SIGN
        | OpCode::ABS
        | OpCode::NEGATE
        | OpCode::INC
        | OpCode::DEC
        | OpCode::NOT
        | OpCode::NZ
        | OpCode::SIZE => 4,
        OpCode::PUSHDATA1
        | OpCode::AND
        | OpCode::OR
        | OpCode::XOR
        | OpCode::ADD
        | OpCode::SUB
        | OpCode::MUL
        | OpCode::DIV
        | OpCode::MOD
        | OpCode::SHL
        | OpCode::SHR
        | OpCode::BOOLAND
        | OpCode::BOOLOR
        | OpCode::NUMEQUAL
        | OpCode::NUMNOTEQUAL
        | OpCode::LT
        | OpCode::LE
        | OpCode::GT
        | OpCode::GE
        | OpCode::MIN
        | OpCode::MAX
        | OpCode::WITHIN
        | OpCode::NEWMAP => 8,
        OpCode::XDROP
        | OpCode::CLEAR
        | OpCode::ROLL
        | OpCode::REVERSEN
        | OpCode::INITSSLOT
        | OpCode::NEWARRAY0
        | OpCode::NEWSTRUCT0
        | OpCode::KEYS
        | OpCode::REMOVE
        | OpCode::CLEARITEMS
        | OpCode::POPITEM => 16,
        OpCode::EQUAL | OpCode::NOTEQUAL | OpCode::MODMUL => 32,
        OpCode::INITSLOT | OpCode::POW | OpCode::SQRT | OpCode::HASKEY | OpCode::PICKITEM => 64,
        OpCode::NEWBUFFER => 256,
        OpCode::PUSHDATA2
        | OpCode::CALL
        | OpCode::CALL_L
        | OpCode::CALLA
        | OpCode::THROW
        | OpCode::NEWARRAY
        | OpCode::NEWARRAY_T
        | OpCode::NEWSTRUCT => 512,
        OpCode::MEMCPY
        | OpCode::CAT
        | OpCode::SUBSTR
        | OpCode::LEFT
        | OpCode::RIGHT
        | OpCode::MODPOW
        | OpCode::PACKMAP
        | OpCode::PACKSTRUCT
        | OpCode::PACK
        | OpCode::UNPACK => 2048,
        OpCode::PUSHDATA4 => 4096,
        OpCode::VALUES
        | OpCode::APPEND
        | OpCode::SETITEM
        | OpCode::REVERSEITEMS
        | OpCode::CONVERT => 8192,
        OpCode::CALLT => 32768,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_price_push_opcodes() {
        // PUSHINT8..PUSHINT64 (0x00-0x03) = 1
        assert_eq!(opcode_price(OpCode::PUSHINT8.byte()), 1, "PUSHINT8");
        assert_eq!(opcode_price(OpCode::PUSHINT16.byte()), 1, "PUSHINT16");
        assert_eq!(opcode_price(OpCode::PUSHINT32.byte()), 1, "PUSHINT32");
        assert_eq!(opcode_price(OpCode::PUSHINT64.byte()), 1, "PUSHINT64");
        // PUSHINT128 (0x04) = 4, PUSHINT256 (0x05) = 4
        assert_eq!(opcode_price(OpCode::PUSHINT128.byte()), 4, "PUSHINT128");
        assert_eq!(opcode_price(OpCode::PUSHINT256.byte()), 4, "PUSHINT256");
    }

    #[test]
    fn opcode_price_flow_control() {
        assert_eq!(opcode_price(OpCode::NOP.byte()), 1, "NOP");
        // Canonical NeoVM short and long conditional jump family.
        for opcode in [
            OpCode::JMP,
            OpCode::JMP_L,
            OpCode::JMPIF,
            OpCode::JMPIF_L,
            OpCode::JMPIFNOT,
            OpCode::JMPIFNOT_L,
            OpCode::JMPEQ,
            OpCode::JMPEQ_L,
            OpCode::JMPNE,
            OpCode::JMPNE_L,
            OpCode::JMPGT,
            OpCode::JMPGT_L,
            OpCode::JMPGE,
            OpCode::JMPGE_L,
            OpCode::JMPLT,
            OpCode::JMPLT_L,
            OpCode::JMPLE,
            OpCode::JMPLE_L,
        ] {
            assert_eq!(opcode_price(opcode.byte()), 2, "JMP-family {opcode}");
        }
        assert_eq!(opcode_price(OpCode::CALL.byte()), 512, "CALL");
        assert_eq!(opcode_price(OpCode::CALLT.byte()), 32768, "CALLT");
        assert_eq!(opcode_price(OpCode::ABORT.byte()), 0, "ABORT");
        assert_eq!(opcode_price(OpCode::RET.byte()), 0, "RET");
        assert_eq!(opcode_price(OpCode::SYSCALL.byte()), 0, "SYSCALL");
    }

    #[test]
    fn opcode_price_expensive_opcodes() {
        assert_eq!(opcode_price(OpCode::VALUES.byte()), 8192, "VALUES");
        assert_eq!(opcode_price(OpCode::CONVERT.byte()), 8192, "CONVERT");
        assert_eq!(opcode_price(OpCode::NEWBUFFER.byte()), 256, "NEWBUFFER");
    }

    #[test]
    fn opcode_price_unknown_defaults_to_max() {
        assert_eq!(opcode_price(0xF0), 65536, "undefined opcode 0xF0");
    }

    #[test]
    fn charge_opcode_deducts_gas() {
        let mut ctx = RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 1_000_000,
            exec_fee_factor_pico: 10_000,
        };
        let mut fee = 0i64;
        charge_opcode(&mut ctx, &mut fee, OpCode::PUSH1.byte()).expect("charge should succeed");
        assert!(ctx.gas_left < 1_000_000, "gas should have decreased");
    }

    #[test]
    fn charge_opcode_insufficient_gas_errors() {
        let mut ctx = RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 10_000,
        };
        let mut fee = 0i64;
        let err = charge_opcode(&mut ctx, &mut fee, OpCode::PUSH1.byte()).unwrap_err();
        assert!(
            err.contains("Insufficient GAS"),
            "error should mention Insufficient GAS: {err}"
        );
    }

    #[test]
    fn instruction_ceiling_permits_counts_below_cap() {
        check_instruction_ceiling(1).expect("count=1 is well below ceiling");
        check_instruction_ceiling(NEO_INSTRUCTION_CEILING - 1)
            .expect("count=ceiling-1 is still below ceiling");
    }

    #[test]
    fn instruction_ceiling_rejects_at_cap() {
        let err = check_instruction_ceiling(NEO_INSTRUCTION_CEILING)
            .expect_err("count=ceiling must trip the watchdog");
        assert!(
            err.contains("instruction ceiling"),
            "error should mention instruction ceiling: {err}"
        );
        assert!(
            err.contains(&NEO_INSTRUCTION_CEILING.to_string()),
            "error should include the ceiling value: {err}"
        );
    }

    #[test]
    fn instruction_ceiling_rejects_above_cap() {
        check_instruction_ceiling(NEO_INSTRUCTION_CEILING + 1_000)
            .expect_err("counts above ceiling must trip the watchdog");
    }

    #[test]
    fn charge_opcode_skips_when_fee_factor_zero() {
        let mut ctx = RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 500,
            exec_fee_factor_pico: 0,
        };
        let mut fee = 0i64;
        charge_opcode(&mut ctx, &mut fee, OpCode::PUSH1.byte())
            .expect("should succeed with zero fee factor");
        assert_eq!(ctx.gas_left, 500, "gas_left should be unchanged");
    }

    #[test]
    fn native_instruction_limit_uses_fee_factor() {
        let ctx = RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 9,
            exec_fee_factor_pico: 30_000,
        };

        assert_eq!(native_instruction_limit(&ctx), 3);
    }

    #[test]
    fn charge_native_instructions_reports_fee() {
        let mut ctx = RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 10,
            exec_fee_factor_pico: 30_000,
        };
        let mut fee = 0;

        charge_native_instructions(&mut ctx, &mut fee, 2).expect("native fee should charge");

        assert_eq!(fee, 60_000);
        assert_eq!(ctx.gas_left, 4);
    }
}
