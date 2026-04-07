use neo_riscv_abi::StackValue;

fn main() {
    let stack = vec![
        StackValue::Integer(8),
        StackValue::ByteString(b"GAS".to_vec()),
    ];
    let bytes = postcard::to_allocvec(&stack).unwrap();
    let decoded: Vec<StackValue> = postcard::from_bytes(&bytes).unwrap();
    println!("{:?}", decoded);
}
