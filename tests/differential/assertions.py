import json
import sys

def assert_state_equality(neovm_state_path, riscv_state_path):
    with open(neovm_state_path, 'r') as f:
        neovm_state = json.load(f)
    
    with open(riscv_state_path, 'r') as f:
        riscv_state = json.load(f)

    # Compare storage
    assert neovm_state.get('storage') == riscv_state.get('storage'), "Storage state mismatch"
    
    # Compare return values
    assert neovm_state.get('return_values') == riscv_state.get('return_values'), "Return values mismatch"
    
    # Compare events
    assert neovm_state.get('events') == riscv_state.get('events'), "Events mismatch"
    
    print("State equality assertions passed.")

if __name__ == "__main__":
    if len(sys.argv) == 3:
        assert_state_equality(sys.argv[1], sys.argv[2])
    else:
        print("Usage: python assertions.py <neovm_state.json> <riscv_state.json>")
