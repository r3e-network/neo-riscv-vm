use neo_riscv_abi::callback_codec::{decode_stack_result, encode_stack_result};
use neo_riscv_abi::{interop_hash, StackValue};

// ---------------------------------------------------------------------------
// Helper: round-trip a single StackValue through encode → decode
// ---------------------------------------------------------------------------
fn round_trip_value(value: StackValue) {
    let original = Ok(vec![value]);
    let bytes = encode_stack_result(&original);
    let decoded = decode_stack_result(&bytes).expect("decode failed");
    assert_eq!(original, Ok(decoded.unwrap()));
}

fn round_trip_stack(stack: Vec<StackValue>) {
    let original = Ok(stack);
    let bytes = encode_stack_result(&original);
    let decoded = decode_stack_result(&bytes).expect("decode failed");
    assert_eq!(original, Ok(decoded.unwrap()));
}

// ===========================================================================
// 1. Round-trip tests for every StackValue variant
// ===========================================================================

// --- Integer ---------------------------------------------------------------

#[test]
fn integer_zero() {
    round_trip_value(StackValue::Integer(0));
}

#[test]
fn integer_positive() {
    round_trip_value(StackValue::Integer(42));
}

#[test]
fn integer_negative() {
    round_trip_value(StackValue::Integer(-1));
}

#[test]
fn integer_max() {
    round_trip_value(StackValue::Integer(i64::MAX));
}

#[test]
fn integer_min() {
    round_trip_value(StackValue::Integer(i64::MIN));
}

// --- BigInteger ------------------------------------------------------------

#[test]
fn biginteger_empty() {
    round_trip_value(StackValue::BigInteger(vec![]));
}

#[test]
fn biginteger_small() {
    round_trip_value(StackValue::BigInteger(vec![0x01, 0x02, 0x03]));
}

#[test]
fn biginteger_large() {
    round_trip_value(StackValue::BigInteger(vec![0xff; 256]));
}

// --- ByteString ------------------------------------------------------------

#[test]
fn bytestring_empty() {
    round_trip_value(StackValue::ByteString(vec![]));
}

#[test]
fn bytestring_hello() {
    round_trip_value(StackValue::ByteString(b"hello".to_vec()));
}

#[test]
fn bytestring_binary() {
    round_trip_value(StackValue::ByteString(vec![0x00, 0xff, 0x80, 0x7f]));
}

// --- Boolean ---------------------------------------------------------------

#[test]
fn boolean_true() {
    round_trip_value(StackValue::Boolean(true));
}

#[test]
fn boolean_false() {
    round_trip_value(StackValue::Boolean(false));
}

// --- Array -----------------------------------------------------------------

#[test]
fn array_empty() {
    round_trip_value(StackValue::Array(vec![]));
}

#[test]
fn array_nested() {
    round_trip_value(StackValue::Array(vec![
        StackValue::Integer(1),
        StackValue::Array(vec![StackValue::Boolean(true), StackValue::Null]),
        StackValue::ByteString(b"inner".to_vec()),
    ]));
}

// --- Struct ----------------------------------------------------------------

#[test]
fn struct_empty() {
    round_trip_value(StackValue::Struct(vec![]));
}

#[test]
fn struct_nested() {
    round_trip_value(StackValue::Struct(vec![
        StackValue::Integer(99),
        StackValue::Struct(vec![StackValue::Boolean(false)]),
    ]));
}

// --- Map -------------------------------------------------------------------

#[test]
fn map_empty() {
    round_trip_value(StackValue::Map(vec![]));
}

#[test]
fn map_with_entries() {
    round_trip_value(StackValue::Map(vec![
        (
            StackValue::ByteString(b"key1".to_vec()),
            StackValue::Integer(100),
        ),
        (
            StackValue::Integer(2),
            StackValue::Array(vec![StackValue::Null]),
        ),
    ]));
}

// --- Interop ---------------------------------------------------------------

#[test]
fn interop_zero() {
    round_trip_value(StackValue::Interop(0));
}

#[test]
fn interop_max() {
    round_trip_value(StackValue::Interop(u64::MAX));
}

// --- Iterator --------------------------------------------------------------

#[test]
fn iterator_zero() {
    round_trip_value(StackValue::Iterator(0));
}

#[test]
fn iterator_42() {
    round_trip_value(StackValue::Iterator(42));
}

// --- Null ------------------------------------------------------------------

#[test]
fn null_value() {
    round_trip_value(StackValue::Null);
}

// --- Pointer ---------------------------------------------------------------

#[test]
fn pointer_zero() {
    round_trip_value(StackValue::Pointer(0));
}

#[test]
fn pointer_negative() {
    round_trip_value(StackValue::Pointer(-1));
}

#[test]
fn pointer_max() {
    round_trip_value(StackValue::Pointer(i64::MAX));
}

// ===========================================================================
// 2. Error result round-trip
// ===========================================================================

#[test]
fn error_result_round_trip() {
    let original: Result<Vec<StackValue>, String> = Err("something went wrong".to_string());
    let bytes = encode_stack_result(&original);
    let decoded = decode_stack_result(&bytes).expect("decode failed");
    assert_eq!(decoded, Err("something went wrong".to_string()));
}

#[test]
fn error_result_empty_message() {
    let original: Result<Vec<StackValue>, String> = Err(String::new());
    let bytes = encode_stack_result(&original);
    let decoded = decode_stack_result(&bytes).expect("decode failed");
    assert_eq!(decoded, Err(String::new()));
}

// ===========================================================================
// 3. interop_hash known values
// ===========================================================================

#[test]
fn interop_hash_platform() {
    // SHA-256("System.Runtime.Platform") = b279fcf6...
    // First 4 bytes read as u32 LE = 0xf6fc79b2 = 4143741362
    assert_eq!(interop_hash("System.Runtime.Platform"), 0xf6fc_79b2);
}

#[test]
fn interop_hash_contract_call() {
    // SHA-256("System.Contract.Call") = 627d5b52...
    // First 4 bytes read as u32 LE = 0x525b7d62 = 1381727586
    assert_eq!(interop_hash("System.Contract.Call"), 0x525b_7d62);
}

#[test]
fn interop_hash_is_sha256_first_4_bytes_le() {
    use sha2::{Digest, Sha256};
    let name = "System.Runtime.Platform";
    let digest = Sha256::digest(name.as_bytes());
    let expected = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]);
    assert_eq!(interop_hash(name), expected);
}

// ===========================================================================
// 4. Edge cases
// ===========================================================================

#[test]
fn empty_stack_round_trip() {
    round_trip_stack(vec![]);
}

#[test]
fn multi_item_stack_round_trip() {
    round_trip_stack(vec![
        StackValue::Integer(1),
        StackValue::Null,
        StackValue::Boolean(true),
        StackValue::ByteString(b"end".to_vec()),
    ]);
}

#[test]
fn truncated_bytes_returns_error() {
    // A valid Integer encoding is tag(0) + 8 bytes; send only 3 bytes.
    let bytes: &[u8] = &[0x00, 0x01, 0x00, 0x00];
    let result = decode_stack_result(bytes);
    assert!(result.is_err(), "truncated input must produce an error");
}

#[test]
fn invalid_tag_byte_returns_error() {
    // Tag 0xFF is not a valid result tag (only 0=Ok, 1=Err).
    let bytes: &[u8] = &[0xFF];
    let result = decode_stack_result(bytes);
    assert!(result.is_err(), "invalid result tag must produce an error");
}

#[test]
fn invalid_stack_value_tag_returns_error() {
    // Result tag 0 (Ok), count = 1, then invalid stack-value tag 0xFE.
    let bytes: &[u8] = &[
        0x00, // Ok
        0x01, 0x00, 0x00, 0x00, // count = 1
        0xFE, // invalid stack value tag
    ];
    let result = decode_stack_result(bytes);
    assert!(result.is_err(), "invalid value tag must produce an error");
}

#[test]
fn trailing_bytes_returns_error() {
    // Encode a valid Null, then append garbage.
    let original = Ok(vec![StackValue::Null]);
    let mut bytes = encode_stack_result(&original);
    bytes.push(0xAB); // trailing garbage
    let result = decode_stack_result(&bytes);
    assert!(result.is_err(), "trailing bytes must produce an error");
}

#[test]
fn completely_empty_input_returns_error() {
    let bytes: &[u8] = &[];
    let result = decode_stack_result(bytes);
    assert!(result.is_err(), "empty input must produce an error");
}

// ===========================================================================
// 5. Decode safety limits (callback_codec)
// ===========================================================================

#[test]
fn decode_rejects_excessive_nesting() {
    // Build a payload: result tag 0 (Ok), top-level count = 1,
    // then 65 nested arrays (tag 4, len 1) to exceed MAX_DECODE_DEPTH (64).
    // decode_stack_value_depth starts at depth=0, each Array recurses with depth+1,
    // so the 65th nested array will attempt depth=65 which exceeds the limit.
    let mut payload = Vec::new();
    payload.push(0x00); // result tag: Ok
    payload.extend_from_slice(&1u32.to_le_bytes()); // top-level stack count = 1
    for _ in 0..65 {
        payload.push(4); // Array tag
        payload.extend_from_slice(&1u32.to_le_bytes()); // array length = 1
    }
    // Innermost value (won't be reached due to depth limit)
    payload.push(9); // Null tag

    let result = decode_stack_result(&payload);
    assert!(result.is_err(), "excessive nesting must be rejected");
    let err = result.unwrap_err();
    assert!(
        err.contains("depth"),
        "error should mention depth, got: {err}"
    );
}

#[test]
fn decode_rejects_excessive_collection_length() {
    // Build a payload: result tag 0 (Ok), top-level count = 5000,
    // which exceeds MAX_COLLECTION_LEN (4096).
    let mut payload = Vec::new();
    payload.push(0x00); // result tag: Ok
    payload.extend_from_slice(&5000u32.to_le_bytes()); // count = 5000

    let result = decode_stack_result(&payload);
    assert!(
        result.is_err(),
        "excessive collection length must be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("length"),
        "error should mention length, got: {err}"
    );
}
