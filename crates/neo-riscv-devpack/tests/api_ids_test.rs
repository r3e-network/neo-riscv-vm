use neo_riscv_abi::interop_hash;
use neo_riscv_devpack::api_ids;

#[test]
fn api_ids_match_interop_hashes() {
    let cases = [
        // System.Storage
        (api_ids::STORAGE_GET_CONTEXT, "System.Storage.GetContext"),
        (
            api_ids::STORAGE_GET_READONLY_CONTEXT,
            "System.Storage.GetReadOnlyContext",
        ),
        (api_ids::STORAGE_AS_READ_ONLY, "System.Storage.AsReadOnly"),
        (api_ids::STORAGE_GET, "System.Storage.Get"),
        (api_ids::STORAGE_PUT, "System.Storage.Put"),
        (api_ids::STORAGE_DELETE, "System.Storage.Delete"),
        (api_ids::STORAGE_FIND, "System.Storage.Find"),
        (api_ids::STORAGE_LOCAL_GET, "System.Storage.Local.Get"),
        (api_ids::STORAGE_LOCAL_PUT, "System.Storage.Local.Put"),
        (api_ids::STORAGE_LOCAL_DELETE, "System.Storage.Local.Delete"),
        (api_ids::STORAGE_LOCAL_FIND, "System.Storage.Local.Find"),
        // System.Contract
        (api_ids::CONTRACT_CALL, "System.Contract.Call"),
        (api_ids::CONTRACT_CREATE, "System.Contract.Create"),
        (api_ids::CONTRACT_UPDATE, "System.Contract.Update"),
        (api_ids::CONTRACT_GET_CALL_FLAGS, "System.Contract.GetCallFlags"),
        (
            api_ids::CONTRACT_CREATE_STANDARD_ACCOUNT,
            "System.Contract.CreateStandardAccount",
        ),
        (
            api_ids::CONTRACT_CREATE_MULTISIG_ACCOUNT,
            "System.Contract.CreateMultisigAccount",
        ),
        (
            api_ids::CONTRACT_NATIVE_ON_PERSIST,
            "System.Contract.NativeOnPersist",
        ),
        (
            api_ids::CONTRACT_NATIVE_POST_PERSIST,
            "System.Contract.NativePostPersist",
        ),
        // System.Runtime
        (api_ids::RUNTIME_PLATFORM, "System.Runtime.Platform"),
        (api_ids::RUNTIME_GET_TRIGGER, "System.Runtime.GetTrigger"),
        (api_ids::RUNTIME_GET_NETWORK, "System.Runtime.GetNetwork"),
        (
            api_ids::RUNTIME_GET_ADDRESS_VERSION,
            "System.Runtime.GetAddressVersion",
        ),
        (
            api_ids::RUNTIME_GET_SCRIPT_CONTAINER,
            "System.Runtime.GetScriptContainer",
        ),
        (
            api_ids::RUNTIME_GET_EXECUTING_SCRIPT_HASH,
            "System.Runtime.GetExecutingScriptHash",
        ),
        (
            api_ids::RUNTIME_GET_CALLING_SCRIPT_HASH,
            "System.Runtime.GetCallingScriptHash",
        ),
        (
            api_ids::RUNTIME_GET_ENTRY_SCRIPT_HASH,
            "System.Runtime.GetEntryScriptHash",
        ),
        (api_ids::RUNTIME_GET_TIME, "System.Runtime.GetTime"),
        (
            api_ids::RUNTIME_GET_INVOCATION_COUNTER,
            "System.Runtime.GetInvocationCounter",
        ),
        (api_ids::RUNTIME_GAS_LEFT, "System.Runtime.GasLeft"),
        (api_ids::RUNTIME_GET_RANDOM, "System.Runtime.GetRandom"),
        (api_ids::RUNTIME_CURRENT_SIGNERS, "System.Runtime.CurrentSigners"),
        (api_ids::RUNTIME_CHECK_WITNESS, "System.Runtime.CheckWitness"),
        (api_ids::RUNTIME_NOTIFY, "System.Runtime.Notify"),
        (api_ids::RUNTIME_LOG, "System.Runtime.Log"),
        (
            api_ids::RUNTIME_GET_NOTIFICATIONS,
            "System.Runtime.GetNotifications",
        ),
        (api_ids::RUNTIME_BURN_GAS, "System.Runtime.BurnGas"),
        (api_ids::RUNTIME_LOAD_SCRIPT, "System.Runtime.LoadScript"),
        // System.Crypto
        (api_ids::CRYPTO_CHECK_SIG, "System.Crypto.CheckSig"),
        (
            api_ids::CRYPTO_CHECK_MULTISIG,
            "System.Crypto.CheckMultisig",
        ),
        // System.Iterator
        (api_ids::ITERATOR_NEXT, "System.Iterator.Next"),
        (api_ids::ITERATOR_VALUE, "System.Iterator.Value"),
        // Backward-compatible alias
        (api_ids::CRYPTO_VERIFY_SIGNATURE, "System.Crypto.CheckSig"),
    ];

    for (actual, name) in cases {
        assert_eq!(actual, interop_hash(name), "{name} hash mismatch");
    }
}
