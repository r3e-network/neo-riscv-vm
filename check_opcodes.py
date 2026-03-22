import re

# Read opcodes defined
with open('crates/neo-riscv-guest/src/opcodes.rs', 'r') as f:
    opcodes_rs = f.read()

defined_opcodes = re.findall(r'pub\(crate\) const ([A-Z0-9_]+): u8', opcodes_rs)

# Read implemented opcodes in lib.rs
with open('crates/neo-riscv-guest/src/lib.rs', 'r') as f:
    lib_rs = f.read()

# Extract the match opcode { ... } block
match_block = re.search(r'match opcode \{(.*?)unsupported opcode', lib_rs, re.DOTALL)
if match_block:
    match_code = match_block.group(1)
    implemented = re.findall(r'([A-Z0-9_]+)\s*(?:\||=>|\.\.)', match_code)
    # clean up dots for ranges
    implemented = [op for op in implemented if op]
else:
    implemented = []
    
missing = set(defined_opcodes) - set(implemented)
print(f"Missing opcodes in match block: {missing}")
