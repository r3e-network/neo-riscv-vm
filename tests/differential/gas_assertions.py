import json
import sys

def assert_gas_parity(neovm_gas, riscv_gas, tolerance_percentage=5.0):
    """
    Asserts that the gas consumed by RISC-V VM is within a tolerable
    range compared to NeoVM execution.
    """
    difference = abs(neovm_gas - riscv_gas)
    allowed_difference = neovm_gas * (tolerance_percentage / 100.0)
    
    assert difference <= allowed_difference, f"Gas mismatch: NeoVM={neovm_gas}, RISC-V={riscv_gas}, Allowed Diff={allowed_difference}"
    print("Gas consumption parity assertion passed.")

if __name__ == "__main__":
    if len(sys.argv) == 3:
        assert_gas_parity(float(sys.argv[1]), float(sys.argv[2]))
    else:
        print("Usage: python gas_assertions.py <neovm_gas_value> <riscv_gas_value>")
