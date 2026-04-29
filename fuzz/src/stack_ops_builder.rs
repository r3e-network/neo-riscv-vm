use alloc::vec::Vec;

pub(crate) fn build_stack_ops_script(_seed: u64, context: &[u8]) -> Vec<u8> {
    let mut script = Vec::new();

    let stack_ops = [
        0x43, 0x45, 0x49, 0x4a, 0x4b, 0x4d, 0x4e, 0x46, 0x48, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55,
        0x06, 0x07,
    ];

    for &byte in context {
        if stack_ops.contains(&byte) {
            script.push(byte);
        }

        if script.len() >= 50 {
            break;
        }
    }

    if script.is_empty() {
        script.push(0x11);
        script.push(0x11);
        script.push(0x4a);
        script.push(0x40);
    } else {
        script.push(0x40);
    }

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use crate::run_script;

    #[test]
    fn default_stack_ops_script_halts() {
        let script = build_stack_ops_script(12345, &[]);
        let result = run_script(&script);
        assert!(result.is_some());
    }

    #[test]
    fn stack_ops_builder_does_not_inject_non_stack_opcodes() {
        let seed = u64::from_le_bytes([0xe5, 0x58, 0x01, 0x00, 0x00, 0x08, 0x58, 0x58]);
        let context = [0x43, 0x00, 0x01, 0x55, 0x37, 0x60, 0xbf, 0xbf, 0x55, 0xa9];

        let script = build_stack_ops_script(seed, &context);

        assert_eq!(script, vec![0x43, 0x55, 0x55, 0x40]);
        assert!(!script.contains(&0x37), "stack_ops script should not inject CALLT");
    }
}
