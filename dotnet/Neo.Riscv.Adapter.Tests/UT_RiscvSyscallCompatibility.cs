using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Cryptography.ECC;
using Neo.Extensions;
using Neo.Network.P2P.Payloads;
using Neo.Persistence;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
[DoNotParallelize]
public class UT_RiscvSyscallCompatibility
{
    [TestMethod]
    public void HardforkGatedSyscallIsRejectedBeforeActivation()
    {
        using var system = new NeoSystem(AdapterTestProtocolSettings.Mainnet, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = new RiscvApplicationEngine(
            TriggerType.Application,
            null,
            snapshot,
            null,
            AdapterTestProtocolSettings.Mainnet,
            ApplicationEngine.TestModeGas,
            new NoopBridge());

        Assert.ThrowsExactly<KeyNotFoundException>(() =>
            NativeRiscvVmBridge.ValidateSyscallForTesting(
                engine,
                ApplicationEngine.System_Storage_Local_Get.Hash,
                CallFlags.All));
    }

    [TestMethod]
    public void SyscallStillRejectsInsufficientCallFlagsAfterActivation()
    {
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = new RiscvApplicationEngine(
            TriggerType.Application,
            null,
            snapshot,
            null,
            AdapterTestProtocolSettings.Default,
            ApplicationEngine.TestModeGas,
            new NoopBridge());

        Assert.ThrowsExactly<InvalidOperationException>(() =>
            NativeRiscvVmBridge.ValidateSyscallForTesting(
                engine,
                ApplicationEngine.System_Storage_Local_Get.Hash,
                CallFlags.None));
    }

    [TestMethod]
    public void HostProfileNameHandlesCallTMarkers()
    {
        Assert.AreEqual("CALLT.42", NativeRiscvVmBridge.GetHostProfileNameForTesting(0x4354002Au));
        Assert.AreEqual(
            ApplicationEngine.System_Runtime_Platform.Name,
            NativeRiscvVmBridge.GetHostProfileNameForTesting(ApplicationEngine.System_Runtime_Platform.Hash));
    }

    [TestMethod]
    public void AllApplicationEngineSyscallsAreHandledByRiscvHostBridge()
    {
        var unsupported = ApplicationEngine.Services.Values
            .OrderBy(descriptor => descriptor.Name)
            .Where(descriptor => !NativeRiscvVmBridge.IsSyscallHandledForTesting(descriptor.Hash))
            .Select(descriptor => descriptor.Name)
            .ToArray();

        CollectionAssert.AreEqual(System.Array.Empty<string>(), unsupported);
        Assert.IsGreaterThanOrEqualTo(41, ApplicationEngine.Services.Count);
    }

    [TestMethod]
    public void NativeSyscallArgumentCountsMatchApplicationEngineDescriptors()
    {
        var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

        using var bridge = new NativeRiscvVmBridge(libraryPath);
        var mismatches = ApplicationEngine.Services.Values
            .OrderBy(descriptor => descriptor.Name)
            .Select(descriptor =>
            {
                var actual = bridge.GetSyscallArgCountForTesting(descriptor.Hash);
                if (actual is null)
                    return "native host library does not export neo_riscv_syscall_arg_count";

                var expected = ExpectedNativeArgumentCount(descriptor);
                return actual.Value == expected
                    ? null
                    : $"{descriptor.Name}: expected {FormatArgumentCount(expected)}, native {FormatArgumentCount(actual.Value)}";
            })
            .Where(mismatch => mismatch is not null)
            .ToArray();

        if (mismatches.Length > 0)
            Assert.Fail(string.Join(Environment.NewLine, mismatches));
    }

    private static nuint ExpectedNativeArgumentCount(InteropDescriptor descriptor)
    {
        return descriptor.Name switch
        {
            "System.Contract.CallNative" => nuint.MaxValue,
            "System.Crypto.CheckMultisig" => nuint.MaxValue,
            _ => (nuint)descriptor.Parameters.Count
        };
    }

    [TestMethod]
    public void ContractCallAcceptsByteStringCallFlags()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            if (string.IsNullOrWhiteSpace(libraryPath))
                Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

            using var bridge = new NativeRiscvVmBridge(libraryPath);
            using var engine = new RiscvApplicationEngine(
                TriggerType.Application,
                null,
                snapshot,
                null,
                AdapterTestProtocolSettings.Default,
                ApplicationEngine.TestModeGas,
                bridge);

            var targetHash = UInt160.Parse("0x0102030405060708090a0b0c0d0e0f1011121314");
            engine.TestingHooks = new ContractCallMock(targetHash, "mock", new Integer(42));

            using var script = new ScriptBuilder();
            script.Emit(OpCode.NEWARRAY0);
            script.EmitPush(new byte[] { (byte)CallFlags.All });
            script.EmitPush("mock");
            script.EmitPush(targetHash);
            script.EmitSysCall(ApplicationEngine.System_Contract_Call);
            script.Emit(OpCode.RET);
            engine.LoadScript(script.ToArray(), configureState: state => state.CallFlags = CallFlags.All);

            Assert.AreEqual(VMState.HALT, engine.Execute());
            Assert.AreEqual(1, engine.ResultStack.Count);
            Assert.AreEqual(42, engine.ResultStack.Pop().GetInteger());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    [TestMethod]
    public void ContractCallAcceptsBufferContractHash()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            if (string.IsNullOrWhiteSpace(libraryPath))
                Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

            using var bridge = new NativeRiscvVmBridge(libraryPath);
            using var engine = new RiscvApplicationEngine(
                TriggerType.Application,
                null,
                snapshot,
                null,
                AdapterTestProtocolSettings.Default,
                ApplicationEngine.TestModeGas,
                bridge);

            var targetHash = UInt160.Parse("0x0102030405060708090a0b0c0d0e0f1011121314");
            engine.TestingHooks = new ContractCallMock(targetHash, "mock", new Integer(42));

            var script = new List<byte>
            {
                (byte)OpCode.NEWARRAY0,
                (byte)OpCode.PUSH15,
                (byte)OpCode.PUSHDATA1,
                4,
            };
            script.AddRange("mock"u8.ToArray());
            script.Add((byte)OpCode.PUSHDATA1);
            script.Add(20);
            script.AddRange(targetHash.GetSpan().ToArray());
            script.Add((byte)OpCode.CONVERT);
            script.Add(0x30);
            script.Add((byte)OpCode.SYSCALL);
            script.AddRange(BitConverter.GetBytes(ApplicationEngine.System_Contract_Call.Hash));
            script.Add((byte)OpCode.RET);

            engine.LoadScript(script.ToArray(), configureState: state => state.CallFlags = CallFlags.All);

            Assert.AreEqual(VMState.HALT, engine.Execute(), engine.FaultException?.ToString());
            Assert.AreEqual(1, engine.ResultStack.Count);
            Assert.AreEqual(42, engine.ResultStack.Pop().GetInteger());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    [TestMethod]
    public void ContractCallPreservesDynamicArgumentOrder()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            if (string.IsNullOrWhiteSpace(libraryPath))
                Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

            using var bridge = new NativeRiscvVmBridge(libraryPath);
            using var engine = new RiscvApplicationEngine(
                TriggerType.Application,
                null,
                snapshot,
                null,
                AdapterTestProtocolSettings.Default,
                ApplicationEngine.TestModeGas,
                bridge);

            var targetHash = UInt160.Parse("0x0102030405060708090a0b0c0d0e0f1011121314");
            engine.TestingHooks = new ContractCallMock(
                targetHash,
                "mock",
                new Integer(42),
                new Integer(128),
                new Integer(1),
                new Integer(381));

            using var script = new ScriptBuilder();
            script.EmitDynamicCall(targetHash, "mock", 128, 1, 381);
            script.Emit(OpCode.RET);
            engine.LoadScript(script.ToArray(), configureState: state => state.CallFlags = CallFlags.All);

            Assert.AreEqual(VMState.HALT, engine.Execute());
            Assert.AreEqual(1, engine.ResultStack.Count);
            Assert.AreEqual(42, engine.ResultStack.Pop().GetInteger());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    [TestMethod]
    public void RuntimeLoadScriptAcceptsByteStringCallFlags()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            if (string.IsNullOrWhiteSpace(libraryPath))
                Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

            using var bridge = new NativeRiscvVmBridge(libraryPath);
            using var engine = new RiscvApplicationEngine(
                TriggerType.Application,
                null,
                snapshot,
                null,
                AdapterTestProtocolSettings.Default,
                ApplicationEngine.TestModeGas,
                bridge);

            var nestedScript = new byte[] { (byte)OpCode.PUSH7, (byte)OpCode.RET };
            using var script = new ScriptBuilder();
            script.Emit(OpCode.NEWARRAY0);
            script.EmitPush(new byte[] { (byte)CallFlags.ReadOnly });
            script.EmitPush(nestedScript);
            script.EmitSysCall(ApplicationEngine.System_Runtime_LoadScript);
            script.Emit(OpCode.RET);
            engine.LoadScript(script.ToArray(), configureState: state => state.CallFlags = CallFlags.All);

            Assert.AreEqual(VMState.HALT, engine.Execute());
            Assert.AreEqual(1, engine.ResultStack.Count);
            Assert.AreEqual(7, engine.ResultStack.Pop().GetInteger());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    [TestMethod]
    public void RuntimeGetCallingScriptHashHonorsNativeCallingScriptHash()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            var libraryPath = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            if (string.IsNullOrWhiteSpace(libraryPath))
                Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

            using var bridge = new NativeRiscvVmBridge(libraryPath);
            using var engine = new RiscvApplicationEngine(
                TriggerType.Application,
                null,
                snapshot,
                null,
                AdapterTestProtocolSettings.Default,
                ApplicationEngine.TestModeGas,
                bridge);

            using var script = new ScriptBuilder();
            script.EmitSysCall(ApplicationEngine.System_Runtime_GetCallingScriptHash);
            script.Emit(OpCode.RET);
            engine.LoadScript(script.ToArray(), configureState: state =>
            {
                state.CallFlags = CallFlags.All;
                typeof(ExecutionContextState)
                    .GetProperty("NativeCallingScriptHash", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                    .SetValue(state, NativeContract.GAS.Hash);
            });

            Assert.AreEqual(VMState.HALT, engine.Execute());
            Assert.AreEqual(1, engine.ResultStack.Count);
            CollectionAssert.AreEqual(
                NativeContract.GAS.Hash.ToArray(),
                engine.ResultStack.Pop().GetSpan().ToArray());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    [TestMethod]
    public void CreateMultisigAccountAcceptsNeoVmArrayArgument()
    {
        RiscvTestEnvironment.RequireNativeRiscvProvider();
        try
        {
            var pubKey = Convert.FromHexString("0278ED78C917797B637A7ED6E7A9D94E8C408444C41EE4C0A0F310A256B9271EDA");
            var expected = Contract.CreateMultiSigRedeemScript(
                1,
                [ECPoint.DecodePoint(pubKey, ECCurve.Secp256r1)]).ToScriptHash();
            using var script = new ScriptBuilder();
            script.EmitPush(pubKey);
            script.EmitPush(1);
            script.Emit(OpCode.PACK);
            script.EmitPush(1);
            script.EmitSysCall(ApplicationEngine.System_Contract_CreateMultisigAccount);
            script.Emit(OpCode.RET);
            var bytes = script.ToArray();

            var riscv = RunCreateMultisigAccountScript(bytes);

            Assert.AreEqual(VMState.HALT, riscv.State, riscv.Fault);
            Assert.AreEqual(1, riscv.Stack.Length);
            CollectionAssert.AreEqual(
                expected.ToArray(),
                riscv.Stack[0].GetSpan().ToArray());
        }
        finally
        {
            RiscvTestEnvironment.RestoreManagedHostProvider();
        }
    }

    private static (VMState State, StackItem[] Stack, string? Fault) RunCreateMultisigAccountScript(byte[] script)
    {
        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = ApplicationEngine.Create(
            TriggerType.Application,
            null,
            snapshot,
            null,
            AdapterTestProtocolSettings.Default,
            ApplicationEngine.TestModeGas);
        engine.LoadScript(script, configureState: state => state.CallFlags = CallFlags.All);

        var state = engine.Execute();
        return (state, engine.ResultStack.ToArray(), engine.FaultException?.ToString());
    }

    private static string FormatArgumentCount(nuint count)
    {
        return count == nuint.MaxValue ? "full-stack" : count.ToString();
    }

    private sealed class ContractCallMock(
        UInt160 expectedHash,
        string expectedMethod,
        StackItem mockResult,
        params StackItem[] expectedArgs)
        : IRiscvApplicationEngineTestingHooks
    {
        public UInt160? OverrideCallingScriptHash(UInt160? current, UInt160? expected) => expected;

        public UInt160? OverrideEntryScriptHash(UInt160? current, UInt160? expected) => expected;

        public bool TryInvokeCustomMock(
            ApplicationEngine engine,
            DataCache snapshot,
            UInt160 contractHash,
            string method,
            StackItem[] args,
            out StackItem result)
        {
            Assert.AreEqual(expectedHash, contractHash);
            Assert.AreEqual(expectedMethod, method);
            Assert.AreEqual(expectedArgs.Length, args.Length);
            for (var index = 0; index < expectedArgs.Length; index++)
                Assert.AreEqual(expectedArgs[index].GetInteger(), args[index].GetInteger());
            result = mockResult;
            return true;
        }

        public void RecordMethodCoverage(UInt160 contractHash, ContractState contractState, ContractMethodDescriptor descriptor)
        {
        }
    }

    private sealed class NoopBridge : IRiscvVmBridge
    {
        public RiscvExecutionResult Execute(RiscvExecutionRequest request)
        {
            return new RiscvExecutionResult(VMState.HALT, [], null);
        }

        public RiscvExecutionResult ExecuteContract(
            ApplicationEngine engine,
            ContractState contract,
            string method,
            CallFlags flags,
            IReadOnlyList<StackItem> args)
        {
            return new RiscvExecutionResult(VMState.HALT, [], null);
        }
    }
}
