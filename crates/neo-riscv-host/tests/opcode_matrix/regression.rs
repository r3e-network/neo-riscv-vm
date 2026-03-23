// Fuzzing regression test capture

use std::fs;
use std::path::Path;

pub fn capture_failing_case(test_name: &str, opcodes: &[u8]) {
    let dir = Path::new("tests/opcode_matrix/regressions");
    fs::create_dir_all(dir).ok();

    let filename = format!("{}/{}.bin", dir.display(), test_name);
    fs::write(&filename, opcodes).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_regression() {
        let opcodes = vec![0x01, 0x02, 0x03];
        capture_failing_case("test_case", &opcodes);
        // Verify file created
    }
}
