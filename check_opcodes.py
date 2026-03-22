import re

# Read opcodes defined
with open('crates/neo-riscv-guest/src/opcodes.rs', 'r') as f:
    opcodes_rs = f.read()

defined_opcodes = set(re.findall(r'pub\(crate\) const ([A-Z0-9_]+): u8', opcodes_rs))

# Read implemented opcodes in lib.rs
with open('crates/neo-riscv-guest/src/lib.rs', 'r') as f:
    lib_rs = f.read()

# Try to find all identifiers matching opcodes in the match block
match_block = re.search(r'match opcode \{(.*?)unsupported opcode', lib_rs, re.DOTALL)
if match_block:
    match_code = match_block.group(1)
    
    implemented = set()
    # It seems range matching like LDLOC0..=LDLOC6 handles LDLOC1-LDLOC5.
    # We should parse opcodes.rs to get values and check if they fall in handled ranges,
    # or just use a more sophisticated approach. 
    # For now, let's just do a naive regex search for all defined opcodes in the whole file
    for opcode in defined_opcodes:
        if re.search(rf'\b{opcode}\b', lib_rs):
            implemented.add(opcode)
            
    # Also check if range is used, e.g. LDLOC0..=LDLOC6 handles LDLOC1 etc.
    ranges = re.findall(r'([A-Z0-9_]+)\.\.=([A-Z0-9_]+)', match_code)
    
    # Simple mapping of prefix to numbers to handle ranges if needed
    for start_op, end_op in ranges:
        prefix = start_op.rstrip('0123456789')
        if prefix:
            try:
                start_idx = int(start_op[len(prefix):])
                end_idx = int(end_op[len(prefix):])
                for i in range(start_idx, end_idx + 1):
                    implemented.add(f"{prefix}{i}")
            except ValueError:
                pass
                
    missing = defined_opcodes - implemented
    print(f"Missing opcodes ({len(missing)}):")
    for op in sorted(missing):
        print(op)
else:
    print("Match block not found.")
