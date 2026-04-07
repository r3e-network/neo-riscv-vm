use crate::RuntimeContext;

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

pub(crate) fn opcode_price(opcode: u8) -> i64 {
    match opcode {
        0x38 | 0x40 | 0x41 | 0xe0 => 0,
        0x00..=0x03 | 0x08 | 0x09 | 0x0b | 0x0f | 0x10..=0x21 | 0x39 | 0xe1 => 1,
        0x22..=0x33 => 2,
        0x43
        | 0x45
        | 0x46
        | 0x4a
        | 0x4b
        | 0x4d
        | 0x4e
        | 0x50
        | 0x51
        | 0x53
        | 0x54
        | 0x58..=0x87
        | 0xd8
        | 0xd9 => 2,
        0x04 | 0x05 | 0x0a | 0x3b..=0x3f | 0x90 | 0x99..=0x9d | 0xaa | 0xb1 | 0xca => 4,
        0x0c | 0x91..=0x93 | 0x9e..=0xa2 | 0xa8 | 0xa9 | 0xab | 0xac | 0xb3..=0xbb | 0xc8 => 8,
        0x48 | 0x49 | 0x52 | 0x55 | 0x56 | 0xc2 | 0xc5 | 0xcc | 0xd2..=0xd4 => 16,
        0x97 | 0x98 | 0xa5 => 32,
        0x57 | 0xa3 | 0xa4 | 0xcb | 0xce => 64,
        0x88 => 256,
        0x0d | 0x34..=0x36 | 0x3a | 0xc3 | 0xc4 | 0xc6 => 512,
        0x89 | 0x8b..=0x8e | 0xa6 | 0xbe..=0xc1 => 2048,
        0x0e => 4096,
        0xcd | 0xcf..=0xd1 | 0xdb => 8192,
        0x37 => 32768,
        _ => 65536,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_price_push_opcodes() {
        // PUSHINT8..PUSHINT64 (0x00-0x03) = 1
        assert_eq!(opcode_price(0x00), 1, "PUSHINT8");
        assert_eq!(opcode_price(0x01), 1, "PUSHINT16");
        assert_eq!(opcode_price(0x02), 1, "PUSHINT32");
        assert_eq!(opcode_price(0x03), 1, "PUSHINT64");
        // PUSHINT128 (0x04) = 4, PUSHINT256 (0x05) = 4
        assert_eq!(opcode_price(0x04), 4, "PUSHINT128");
        assert_eq!(opcode_price(0x05), 4, "PUSHINT256");
    }

    #[test]
    fn opcode_price_flow_control() {
        assert_eq!(opcode_price(0x21), 1, "NOP");
        // JMP range 0x22..=0x33 = 2
        for op in 0x22..=0x33u8 {
            assert_eq!(opcode_price(op), 2, "JMP-family 0x{op:02x}");
        }
        assert_eq!(opcode_price(0x34), 512, "CALL");
        assert_eq!(opcode_price(0x37), 32768, "CALLT");
        assert_eq!(opcode_price(0x38), 0, "ABORT");
        assert_eq!(opcode_price(0x40), 0, "RET");
        assert_eq!(opcode_price(0x41), 0, "SYSCALL");
    }

    #[test]
    fn opcode_price_expensive_opcodes() {
        assert_eq!(opcode_price(0xcd), 8192, "VALUES");
        assert_eq!(opcode_price(0xdb), 8192, "CONVERT");
        assert_eq!(opcode_price(0x88), 256, "NEWBUFFER");
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
        // PUSH1 = opcode 0x11, price = 1
        charge_opcode(&mut ctx, &mut fee, 0x11).expect("charge should succeed");
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
        let err = charge_opcode(&mut ctx, &mut fee, 0x11).unwrap_err();
        assert!(
            err.contains("Insufficient GAS"),
            "error should mention Insufficient GAS: {err}"
        );
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
        charge_opcode(&mut ctx, &mut fee, 0x11).expect("should succeed with zero fee factor");
        assert_eq!(ctx.gas_left, 500, "gas_left should be unchanged");
    }
}
