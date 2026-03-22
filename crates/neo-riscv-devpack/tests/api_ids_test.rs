use neo_riscv_abi::interop_hash;
use neo_riscv_devpack::api_ids;

#[test]
fn api_ids_match_interop_hashes() {
    let cases = [
        (api_ids::STORAGE_GET, "System.Storage.Get"),
        (api_ids::STORAGE_PUT, "System.Storage.Put"),
        (api_ids::STORAGE_DELETE, "System.Storage.Delete"),
        (api_ids::CONTRACT_CALL, "System.Contract.Call"),
        (api_ids::CONTRACT_CREATE, "System.Contract.Create"),
        (api_ids::CONTRACT_UPDATE, "System.Contract.Update"),
        (api_ids::RUNTIME_NOTIFY, "System.Runtime.Notify"),
        (api_ids::RUNTIME_LOG, "System.Runtime.Log"),
        (api_ids::RUNTIME_CHECK_WITNESS, "System.Runtime.CheckWitness"),
        (api_ids::CRYPTO_VERIFY_SIGNATURE, "System.Crypto.CheckSig"),
    ];

    for (actual, name) in cases {
        assert_eq!(actual, interop_hash(name), "{name} hash mismatch");
    }
}
