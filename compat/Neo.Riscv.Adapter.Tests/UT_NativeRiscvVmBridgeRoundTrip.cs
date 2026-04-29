using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Cryptography.BLS12_381;
using Neo.Extensions;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using VmPointer = Neo.VM.Types.Pointer;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
[DoNotParallelize]
public class UT_NativeRiscvVmBridgeRoundTrip
{
    private const string G1Hex =
        "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";

    [TestMethod]
    public void StorageContextRejectsGuestFabricatedArray()
    {
        var forged = new Neo.VM.Types.Array(new StackItem[]
        {
            new Integer(123),
            StackItem.False,
        });

        Assert.IsFalse(TryParseStorageContext(forged, out _));
    }

    [TestMethod]
    public void StorageContextAcceptsOpaqueInteropHandle()
    {
        var expected = new StorageContext
        {
            Id = 123,
            IsReadOnly = true,
        };

        Assert.IsTrue(TryParseStorageContext(StackItem.FromInterface(expected), out var actual));
        Assert.AreEqual(expected.Id, actual.Id);
        Assert.AreEqual(expected.IsReadOnly, actual.IsReadOnly);
    }

    [TestMethod]
    public void VoidContractCallConsumesArgumentsWithoutPushingNull()
    {
        var keep = new Integer(99);
        var inputStack = new StackItem[]
        {
            keep,
            new ByteString(UInt160.Zero.ToArray()),
            new ByteString("method"u8.ToArray()),
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
        };

        var next = NativeRiscvVmBridge.BuildContractCallReturnStack(
            inputStack,
            consumedArgumentCount: 4,
            ContractParameterType.Void,
            System.Array.Empty<StackItem>());

        Assert.AreEqual(1, next.Length);
        Assert.AreSame(keep, next[0]);
    }

    [TestMethod]
    public void NonVoidContractCallPushesActualResultOnly()
    {
        var keep = new Integer(99);
        var result = new Integer(42);
        var inputStack = new StackItem[]
        {
            keep,
            new ByteString(UInt160.Zero.ToArray()),
            new ByteString("method"u8.ToArray()),
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
        };

        var next = NativeRiscvVmBridge.BuildContractCallReturnStack(
            inputStack,
            consumedArgumentCount: 4,
            ContractParameterType.Integer,
            new StackItem[] { result });

        Assert.AreEqual(2, next.Length);
        Assert.AreSame(keep, next[0]);
        Assert.AreSame(result, next[1]);
    }

    [TestMethod]
    public void DynamicContractCallPushesNullForEmptyResult()
    {
        var keep = new Integer(99);
        var inputStack = new StackItem[]
        {
            keep,
            new ByteString(UInt160.Zero.ToArray()),
            new ByteString("method"u8.ToArray()),
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
        };

        var next = NativeRiscvVmBridge.BuildDynamicContractCallReturnStack(
            inputStack,
            consumedArgumentCount: 4,
            System.Array.Empty<StackItem>());

        Assert.AreEqual(2, next.Length);
        Assert.AreSame(keep, next[0]);
        Assert.AreSame(StackItem.Null, next[1]);
    }

    [TestMethod]
    public void DynamicContractCallRejectsMultipleResults()
    {
        var inputStack = new StackItem[]
        {
            new ByteString(UInt160.Zero.ToArray()),
            new ByteString("method"u8.ToArray()),
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
        };

        Assert.ThrowsExactly<NotSupportedException>(() =>
            NativeRiscvVmBridge.BuildDynamicContractCallReturnStack(
                inputStack,
                consumedArgumentCount: 4,
                new StackItem[] { new Integer(1), new Integer(2) }));
    }

    [TestMethod]
    public void DynamicRuntimeLoadScriptPushesNullForEmptyResult()
    {
        var keep = new Integer(99);
        var inputStack = new StackItem[]
        {
            keep,
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
            new ByteString(System.Array.Empty<byte>()),
        };

        var next = NativeRiscvVmBridge.BuildDynamicCallReturnStack(
            inputStack,
            consumedArgumentCount: 3,
            System.Array.Empty<StackItem>());

        Assert.AreEqual(2, next.Length);
        Assert.AreSame(keep, next[0]);
        Assert.AreSame(StackItem.Null, next[1]);
    }

    [TestMethod]
    public void DynamicRuntimeLoadScriptRejectsMultipleResults()
    {
        var inputStack = new StackItem[]
        {
            new Neo.VM.Types.Array(System.Array.Empty<StackItem>()),
            new Integer((int)CallFlags.All),
            new ByteString(System.Array.Empty<byte>()),
        };

        Assert.ThrowsExactly<NotSupportedException>(() =>
            NativeRiscvVmBridge.BuildDynamicCallReturnStack(
                inputStack,
                consumedArgumentCount: 3,
                new StackItem[] { new Integer(1), new Integer(2) }));
    }

    [TestMethod]
    public void NonIteratorInteropRoundTripsThroughNativeStack()
    {
        using var bridge = CreateBridge();
        var scope = CreateExecutionScope();
        var point = CryptoLib.Bls12381Deserialize(G1Hex.HexToBytes());

        var roundTripped = RoundTripSingleItem(bridge, scope, point);

        Assert.IsInstanceOfType<InteropInterface>(roundTripped);
        var actual = ((InteropInterface)roundTripped).GetInterface<G1Affine>().ToCompressed().ToHexString();
        Assert.AreEqual(G1Hex, actual);
    }

    [TestMethod]
    public void ContractStateRoundTripsThroughNativeStack()
    {
        using var bridge = CreateBridge();
        var scope = CreateExecutionScope();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();

        var contract = NativeContract.ContractManagement.GetContract(snapshot, NativeContract.NEO.Hash);
        Assert.IsNotNull(contract);

        var roundTripped = RoundTripSingleItem(bridge, scope, contract!.ToStackItem(null));

        var restored = (ContractState)RuntimeHelpers.GetUninitializedObject(typeof(ContractState));
        ((IInteroperable)restored).FromStackItem(roundTripped);

        Assert.AreEqual(contract.ToJson().ToString(), restored.ToJson().ToString());
    }

    [TestMethod]
    public void BufferRoundTripsThroughNativeStack()
    {
        using var bridge = CreateBridge();
        var scope = CreateExecutionScope();

        var roundTripped = RoundTripSingleItem(bridge, scope, new Neo.VM.Types.Buffer([0x01, 0x02, 0x03]));

        Assert.IsInstanceOfType<Neo.VM.Types.Buffer>(roundTripped);
        CollectionAssert.AreEqual(new byte[] { 0x01, 0x02, 0x03 }, roundTripped.GetSpan().ToArray());
    }

    [TestMethod]
    public void PointerRoundTripUsesCurrentScript()
    {
        using var bridge = CreateBridge();
        var scope = CreateExecutionScope();
        var script = new Script(new byte[] { (byte)OpCode.NOP, (byte)OpCode.RET }, false);
        SetCurrentScript(scope, script);

        var roundTripped = RoundTripSingleItem(bridge, scope, new VmPointer(script, 1));

        Assert.IsInstanceOfType<VmPointer>(roundTripped);
        var pointer = (VmPointer)roundTripped;
        Assert.AreSame(script, pointer.Script);
        Assert.AreEqual(1, pointer.Position);
    }

    [TestMethod]
    public void PointerRoundTripCanRemapInitializeWrapperCoordinates()
    {
        using var bridge = CreateBridge();
        var scope = CreateExecutionScope();
        var wrapperScript = new Script(new byte[] { (byte)OpCode.CALL_L, 0, 0, 0, 0, (byte)OpCode.JMP_L, 0, 0, 0, 0, (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.RET }, false);
        var originalScript = new Script(new byte[] { (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.RET }, false);
        SetCurrentScript(scope, wrapperScript);
        SetPointerScript(scope, originalScript);
        SetPointerPositionDelta(scope, -10);

        var roundTripped = RoundTripSingleItem(bridge, scope, new VmPointer(wrapperScript, 12));

        Assert.IsInstanceOfType<VmPointer>(roundTripped);
        var pointer = (VmPointer)roundTripped;
        Assert.AreSame(originalScript, pointer.Script);
        Assert.AreEqual(2, pointer.Position);
    }

    [TestMethod]
    public void ContractManagementGetContractExecutionRoundTripsAllNativeContractStates()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        var snapshot = system.GetSnapshotCache().CloneCache();
        try
        {
            foreach (var contract in NativeContract.Contracts)
            {
                ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
                using var engine = ApplicationEngine.Create(
                    TriggerType.Application,
                    null,
                    snapshot,
                    CreatePersistingBlock(),
                    settings: AdapterTestProtocolSettings.Default);
                using var script = new ScriptBuilder();

                script.EmitDynamicCall(NativeContract.ContractManagement.Hash, "getContract", contract.Hash);
                engine.LoadScript(script.ToArray());

                Assert.AreEqual(VMState.HALT, engine.Execute(), contract.Name);
                Assert.AreEqual(1, engine.ResultStack.Count, contract.Name);

                var result = engine.ResultStack.Pop();
                Assert.IsInstanceOfType<Neo.VM.Types.Array>(result, contract.Name);

                var expected = NativeContract.ContractManagement.GetContract(snapshot, contract.Hash);
                Assert.IsNotNull(expected, contract.Name);
                AssertStackItemEquivalent(expected!.ToStackItem(null), result, contract.Name);

                var restored = (ContractState)RuntimeHelpers.GetUninitializedObject(typeof(ContractState));
                ((IInteroperable)restored).FromStackItem(result);

                Assert.AreEqual(expected.ToJson().ToString(), restored.ToJson().ToString(), contract.Name);
            }
        }
        finally
        {
            ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        }
    }

    [TestMethod]
    public void FaultIpPropagates_AbortAtNonZeroOffset()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        var snapshot = system.GetSnapshotCache().CloneCache();
        using var engine = ApplicationEngine.Create(
            TriggerType.Application,
            null,
            snapshot,
            CreatePersistingBlock(),
            settings: AdapterTestProtocolSettings.Default);

        // Script layout: [NOP] [NOP] [NOP] [NOP] [NOP] [ABORT] — ABORT lives at offset 5.
        // Guest interpreter must report fault_ip = 5 and the adapter must write it onto
        // CurrentContext.InstructionPointer via the side-channel FFI + reflection path.
        var script = new byte[] { (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.NOP, (byte)OpCode.ABORT };
        engine.LoadScript(script);

        Assert.AreEqual(VMState.FAULT, engine.Execute());
        Assert.IsNotNull(engine.CurrentContext);
        Assert.AreEqual(5, engine.CurrentContext!.InstructionPointer,
            "fault IP must match the ABORT opcode's offset in the script");
    }

    [TestMethod]
    public void FaultIpPropagates_AssertAtNonZeroOffset()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        var snapshot = system.GetSnapshotCache().CloneCache();
        using var engine = ApplicationEngine.Create(
            TriggerType.Application,
            null,
            snapshot,
            CreatePersistingBlock(),
            settings: AdapterTestProtocolSettings.Default);

        // Script: [PUSHF] [ASSERT] — PUSHF pushes false, then ASSERT on a false value FAULTs.
        // ASSERT lives at offset 1.
        var script = new byte[] { (byte)OpCode.PUSHF, (byte)OpCode.ASSERT };
        engine.LoadScript(script);

        Assert.AreEqual(VMState.FAULT, engine.Execute());
        Assert.IsNotNull(engine.CurrentContext);
        Assert.AreEqual(1, engine.CurrentContext!.InstructionPointer,
            "fault IP must match the ASSERT opcode's offset in the script");
    }

    private static NativeRiscvVmBridge CreateBridge()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

        return new NativeRiscvVmBridge(libraryPath!);
    }

    private static string? ResolveWorkspaceLibraryPath()
    {
        var baseDirectory = AppContext.BaseDirectory;
        var release = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "release", "libneo_riscv_host.so"));
        if (File.Exists(release))
            return release;

        var debug = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "debug", "libneo_riscv_host.so"));
        if (File.Exists(debug))
            return debug;

        return null;
    }

    private static bool TryParseStorageContext(StackItem item, out StorageContext context)
    {
        var method = typeof(NativeRiscvVmBridge).GetMethod("TryParseStorageContextItem", BindingFlags.Static | BindingFlags.NonPublic);
        Assert.IsNotNull(method);

        object?[] args = [item, null];
        var result = (bool)method!.Invoke(null, args)!;
        context = args[1] is StorageContext parsed ? parsed : new StorageContext();
        return result;
    }

    private static object CreateExecutionScope()
    {
        var scopeType = typeof(NativeRiscvVmBridge).GetNestedType("ExecutionScope", BindingFlags.NonPublic);
        Assert.IsNotNull(scopeType);
        return Activator.CreateInstance(scopeType!, nonPublic: true)!;
    }

    private static void SetCurrentScript(object scope, Script script)
    {
        var property = scope.GetType().GetProperty("CurrentScript", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        Assert.IsNotNull(property);
        property!.SetValue(scope, script);
    }

    private static void SetPointerScript(object scope, Script script)
    {
        var property = scope.GetType().GetProperty("PointerScript", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        Assert.IsNotNull(property);
        property!.SetValue(scope, script);
    }

    private static void SetPointerPositionDelta(object scope, int delta)
    {
        var property = scope.GetType().GetProperty("PointerPositionDelta", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        Assert.IsNotNull(property);
        property!.SetValue(scope, delta);
    }

    private static StackItem RoundTripSingleItem(NativeRiscvVmBridge bridge, object scope, StackItem item)
    {
        var result = CreateNativeHostResult(bridge, [item], scope);
        try
        {
            var stack = ReadStack(bridge, result, scope);
            Assert.AreEqual(1, stack.Length);
            return stack[0];
        }
        finally
        {
            FreeNativeHostResult(result);
        }
    }

    private static object CreateNativeHostResult(NativeRiscvVmBridge bridge, StackItem[] stack, object scope)
    {
        var method = typeof(NativeRiscvVmBridge).GetMethod("CreateNativeHostResult", BindingFlags.Instance | BindingFlags.NonPublic);
        Assert.IsNotNull(method);
        return method!.Invoke(bridge, [stack, scope])!;
    }

    private static StackItem[] ReadStack(NativeRiscvVmBridge bridge, object nativeHostResult, object scope)
    {
        var nativeType = nativeHostResult.GetType();
        var stackPtr = (IntPtr)nativeType.GetField("StackPtr", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(nativeHostResult)!;
        var stackLen = (UIntPtr)nativeType.GetField("StackLen", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(nativeHostResult)!;

        var method = typeof(NativeRiscvVmBridge).GetMethod("ReadStack", BindingFlags.Instance | BindingFlags.NonPublic);
        Assert.IsNotNull(method);
        return (StackItem[])method!.Invoke(bridge, [stackPtr, stackLen, null, scope, true])!;
    }

    private static void FreeNativeHostResult(object nativeHostResult)
    {
        var nativeType = nativeHostResult.GetType();
        var stackPtr = (IntPtr)nativeType.GetField("StackPtr", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(nativeHostResult)!;
        var stackLen = (UIntPtr)nativeType.GetField("StackLen", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(nativeHostResult)!;
        var errorPtr = (IntPtr)nativeType.GetField("ErrorPtr", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(nativeHostResult)!;

        if (stackPtr != IntPtr.Zero)
        {
            var freeMethod = typeof(NativeRiscvVmBridge).GetMethod("FreeNativeStackItems", BindingFlags.Static | BindingFlags.NonPublic);
            Assert.IsNotNull(freeMethod);
            freeMethod!.Invoke(null, [stackPtr, checked((int)stackLen)]);
        }

        if (errorPtr != IntPtr.Zero)
        {
            System.Runtime.InteropServices.Marshal.FreeHGlobal(errorPtr);
        }
    }

    private static Block CreatePersistingBlock()
    {
        return new Block
        {
            Header = new Header
            {
                Index = 1,
                PrevHash = UInt256.Zero,
                MerkleRoot = UInt256.Zero,
                NextConsensus = UInt160.Zero,
                Witness = Witness.Empty,
            },
            Transactions = [],
        };
    }

    private static void AssertStackItemEquivalent(StackItem expected, StackItem actual, string rootPath)
    {
        if (expected.GetType() != actual.GetType())
        {
            Assert.Fail($"{rootPath}: expected {expected.GetType().Name}, got {actual.GetType().Name}");
        }

        switch (expected)
        {
            case Integer expectedInteger:
                Assert.AreEqual(expectedInteger.GetInteger(), ((Integer)actual).GetInteger(), rootPath);
                break;
            case ByteString expectedByteString:
            {
                var expectedBytes = expectedByteString.GetSpan().ToArray();
                var actualBytes = ((ByteString)actual).GetSpan().ToArray();
                if (!expectedBytes.SequenceEqual(actualBytes))
                {
                    Assert.Fail(
                        $"{rootPath}: expected bytes {Convert.ToHexString(expectedBytes)}, got {Convert.ToHexString(actualBytes)}");
                }
                break;
            }
            case Neo.VM.Types.Buffer expectedBuffer:
                CollectionAssert.AreEqual(expectedBuffer.GetSpan().ToArray(), ((Neo.VM.Types.Buffer)actual).GetSpan().ToArray(), rootPath);
                break;
            case Neo.VM.Types.Boolean expectedBoolean:
                Assert.AreEqual(expectedBoolean.GetBoolean(), ((Neo.VM.Types.Boolean)actual).GetBoolean(), rootPath);
                break;
            case Null:
                break;
            case Struct expectedStruct:
            {
                var actualStruct = (Struct)actual;
                Assert.AreEqual(expectedStruct.Count, actualStruct.Count, rootPath);
                for (var i = 0; i < expectedStruct.Count; i++)
                {
                    AssertStackItemEquivalent(expectedStruct[i], actualStruct[i], $"{rootPath}[{i}]");
                }
                break;
            }
            case Neo.VM.Types.Array expectedArray:
            {
                var actualArray = (Neo.VM.Types.Array)actual;
                Assert.AreEqual(expectedArray.Count, actualArray.Count, rootPath);
                for (var i = 0; i < expectedArray.Count; i++)
                {
                    AssertStackItemEquivalent(expectedArray[i], actualArray[i], $"{rootPath}[{i}]");
                }
                break;
            }
            case Map expectedMap:
            {
                var actualMap = (Map)actual;
                Assert.AreEqual(expectedMap.Count, actualMap.Count, rootPath);
                for (var i = 0; i < expectedMap.Count; i++)
                {
                    var expectedEntry = expectedMap.ElementAt(i);
                    var actualEntry = actualMap.ElementAt(i);
                    AssertStackItemEquivalent(expectedEntry.Key, actualEntry.Key, $"{rootPath}.key[{i}]");
                    AssertStackItemEquivalent(expectedEntry.Value, actualEntry.Value, $"{rootPath}.value[{i}]");
                }
                break;
            }
            default:
                Assert.Fail($"{rootPath}: unsupported comparison type {expected.GetType().FullName}");
                break;
        }
    }
}
