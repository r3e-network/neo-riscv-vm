using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Cryptography;
using Neo.Cryptography.ECC;
using Neo.Extensions;
using Neo.Network.P2P;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;
using System;
using System.Numerics;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
[DoNotParallelize]
public class UT_VerificationMultiSigCompatibility
{
    private string? _previousLibraryPath;

    [TestInitialize]
    public void TestSetup()
    {
        _previousLibraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        RiscvApplicationEngineProviderResolver.ResetForTesting();
    }

    [TestCleanup]
    public void TestCleanup()
    {
        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, _previousLibraryPath);
        RiscvApplicationEngineProviderResolver.ResetForTesting();
    }

    [TestMethod]
    public void ContractCallVerifyWithECDsa_ResultFeedsIntoLdlocContinuation()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set.");

        var privateKey = "7177f0d04c79fa0b8c91fe90c1cf1d44772d1fba6e5eb9b281a22cd3aafb51fe".HexToBytes();
        var publicKey = ECPoint.Parse(
            "04fd0a8c1ce5ae5570fdd46e7599c16b175bf0ebdfe9c178f1ab848fb16dac74a5" +
            "d301b0534c7bcf1b3760881f0c420d17084907edd771e1c9c8e941bbf6ff9108",
            ECCurve.Secp256k1);
        var message = "48656C6C6F576F726C64".HexToBytes();
        var signature = Crypto.Sign(message, privateKey, ECCurve.Secp256k1, HashAlgorithm.Keccak256);

        RiscvApplicationEngineProviderResolver.ResetForTesting();
        var riscv = ExecuteContinuationScript(useRiscvProvider: true, message, signature, publicKey.EncodePoint(true));
        Assert.IsTrue(riscv, "RISC-V-backed execution should preserve the direct verifyWithECDsa continuation.");
    }

    [TestMethod]
    public void ContractCallVerifyWithECDsa_TransactionBackedMessageMatchesBaseline()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set.");

        var privateKey = "7177f0d04c79fa0b8c91fe90c1cf1d44772d1fba6e5eb9b281a22cd3aafb51fe".HexToBytes();
        var publicKey = ECPoint.Parse(
            "04fd0a8c1ce5ae5570fdd46e7599c16b175bf0ebdfe9c178f1ab848fb16dac74a5" +
            "d301b0534c7bcf1b3760881f0c420d17084907edd771e1c9c8e941bbf6ff9108",
            ECCurve.Secp256k1);
        var tx = new Transaction
        {
            Script = new byte[] { (byte)OpCode.PUSH2 },
            Attributes = [],
            Signers =
            [
                new Signer
                {
                    Account = UInt160.Zero,
                    Scopes = WitnessScope.CalledByEntry,
                    AllowedContracts = [],
                    AllowedGroups = [],
                    Rules = [],
                }
            ],
            Witnesses = [Witness.Empty]
        };
        var signature = Crypto.Sign(
            tx.GetSignData(AdapterTestProtocolSettings.Default.Network),
            privateKey,
            ECCurve.Secp256k1,
            HashAlgorithm.Keccak256);
        Assert.IsTrue(
            Crypto.VerifySignature(tx.GetSignData(AdapterTestProtocolSettings.Default.Network), signature, publicKey, HashAlgorithm.Keccak256),
            "The direct transaction sign-data precondition must verify before exercising the VM path.");

        RiscvApplicationEngineProviderResolver.ResetForTesting();
        var riscv = ExecuteTransactionVerifyScript(
            useRiscvProvider: true,
            tx,
            signature,
            publicKey.EncodePoint(true));
        Assert.IsTrue(riscv, "RISC-V-backed execution should accept the transaction-backed verifyWithECDsa call.");
    }

    private static bool ExecuteContinuationScript(
        bool useRiscvProvider,
        byte[] message,
        byte[] signature,
        byte[] publicKey)
    {
        ApplicationEngine.Provider = useRiscvProvider
            ? RiscvApplicationEngineProviderResolver.ResolveRequiredProvider()
            : null;

        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var script = new ScriptBuilder();

        script.Emit(OpCode.INITSLOT, new ReadOnlySpan<byte>([4, 0]));
        script.Emit(OpCode.PUSH0, OpCode.STLOC3);
        script.EmitPush((byte)NamedCurveHash.secp256k1Keccak256);
        script.EmitPush(signature);
        script.EmitPush(publicKey);
        script.EmitPush(message);
        script.Emit(OpCode.PUSH4, OpCode.PACK);
        EmitAppCallNoArgs(script, NativeContract.CryptoLib.Hash, "verifyWithECDsa", CallFlags.None);
        script.Emit(OpCode.LDLOC3, OpCode.ADD, OpCode.STLOC3, OpCode.LDLOC3, OpCode.RET);

        using var engine = ApplicationEngine.Create(
            TriggerType.Application,
            null,
            snapshot,
            settings: AdapterTestProtocolSettings.Default);
        engine.LoadScript(script.ToArray());

        if (engine.Execute() != VMState.HALT || engine.ResultStack.Count == 0)
            return false;

        return engine.ResultStack.Pop().GetInteger() == BigInteger.One;
    }

    private static bool ExecuteTransactionVerifyScript(
        bool useRiscvProvider,
        Transaction tx,
        byte[] signature,
        byte[] publicKey)
    {
        ApplicationEngine.Provider = useRiscvProvider
            ? RiscvApplicationEngineProviderResolver.ResolveRequiredProvider()
            : null;

        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var invocation = new ScriptBuilder();
        invocation.EmitPush(signature);

        using var verification = new ScriptBuilder();
        verification.EmitPush(1);
        verification.EmitPush(publicKey);
        verification.EmitPush(1);
        verification.Emit(OpCode.INITSLOT, new ReadOnlySpan<byte>([7, 0]));
        verification.Emit(OpCode.STLOC5);
        verification.Emit(OpCode.LDLOC5, OpCode.PACK, OpCode.STLOC1);
        verification.Emit(OpCode.STLOC6);
        verification.Emit(OpCode.DEPTH);
        verification.Emit(OpCode.LDLOC6);
        verification.Emit(OpCode.JMPEQ, new ReadOnlySpan<byte>([0]));
        var checkEndOffset = verification.Length;
        verification.Emit(OpCode.ABORT);
        var checkStartOffset = verification.Length;
        verification.Emit(OpCode.LDLOC6);
        verification.Emit(OpCode.PACK, OpCode.STLOC0);
        verification.EmitSysCall(ApplicationEngine.System_Runtime_GetNetwork);
        verification.EmitPush(0x100000000);
        verification.Emit(OpCode.ADD, OpCode.PUSH4, OpCode.LEFT);
        verification.EmitSysCall(ApplicationEngine.System_Runtime_GetScriptContainer);
        verification.Emit(OpCode.PUSH0, OpCode.PICKITEM);
        verification.Emit(OpCode.CAT, OpCode.STLOC2);
        verification.Emit(OpCode.PUSH0, OpCode.STLOC3);
        verification.Emit(OpCode.PUSH0, OpCode.STLOC4);
        verification.EmitPush((byte)NamedCurveHash.secp256k1Keccak256);
        verification.Emit(
            OpCode.LDLOC0, OpCode.LDLOC3, OpCode.PICKITEM,
            OpCode.LDLOC1, OpCode.LDLOC4, OpCode.PICKITEM,
            OpCode.LDLOC2,
            OpCode.PUSH4, OpCode.PACK);
        EmitAppCallNoArgs(verification, NativeContract.CryptoLib.Hash, "verifyWithECDsa", CallFlags.None);
        verification.Emit(OpCode.RET);

        var verificationScript = verification.ToArray();
        verificationScript[checkEndOffset - 1] = (byte)(checkStartOffset - checkEndOffset + 2);

        using var engine = ApplicationEngine.Create(
            TriggerType.Verification,
            tx,
            snapshot,
            settings: AdapterTestProtocolSettings.Default);
        engine.LoadScript(verificationScript, configureState: p => p.CallFlags = CallFlags.ReadOnly);
        engine.LoadScript(new Script(invocation.ToArray(), true), configureState: p => p.CallFlags = CallFlags.None);

        if (engine.Execute() != VMState.HALT || engine.ResultStack.Count == 0)
            return false;

        return engine.ResultStack.Pop().GetBoolean();
    }

    private static ScriptBuilder EmitAppCallNoArgs(
        ScriptBuilder builder,
        UInt160 contractHash,
        string method,
        CallFlags flags)
    {
        builder.EmitPush((byte)flags);
        builder.EmitPush(method);
        builder.EmitPush(contractHash);
        builder.EmitSysCall(ApplicationEngine.System_Contract_Call);
        return builder;
    }
}
