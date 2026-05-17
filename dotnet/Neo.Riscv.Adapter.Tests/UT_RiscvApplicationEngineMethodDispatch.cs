using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System.Collections.Generic;
using System.Linq;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
[DoNotParallelize]
public class UT_RiscvApplicationEngineMethodDispatch
{
    [TestMethod]
    public void DirectRiscVExecutionRequestCarriesLoadedMethodName()
    {
        var bridge = new CapturingBridge();
        var previousProvider = ApplicationEngine.Provider;
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();

        try
        {
            ApplicationEngine.Provider = new RiscvApplicationEngineProvider(bridge);
            using var engine = (RiscvApplicationEngine)ApplicationEngine.Create(
                TriggerType.Application,
                null,
                snapshot,
                settings: AdapterTestProtocolSettings.Default,
                gas: ApplicationEngine.TestModeGas);
            var contract = CreateRiscVContract();
            var method = contract.Manifest.Abi.GetMethod("main", 0);

            Assert.IsNotNull(method);
            engine.LoadContract(contract, method!, CallFlags.All);
            var state = engine.Execute();

            Assert.AreEqual(VMState.HALT, state);
            Assert.IsNotNull(bridge.LastRequest);
            Assert.AreEqual("main", bridge.LastRequest!.Method);
        }
        finally
        {
            ApplicationEngine.Provider = previousProvider;
        }
    }

    [TestMethod]
    public void DirectRiscVExecutionRequestCarriesArgumentsBottomToTop()
    {
        var bridge = new CapturingBridge();
        var previousProvider = ApplicationEngine.Provider;
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();

        try
        {
            ApplicationEngine.Provider = new RiscvApplicationEngineProvider(bridge);
            using var engine = (RiscvApplicationEngine)ApplicationEngine.Create(
                TriggerType.Application,
                null,
                snapshot,
                settings: AdapterTestProtocolSettings.Default,
                gas: ApplicationEngine.TestModeGas);
            var contract = CreateRiscVContract("main", 3);
            var method = contract.Manifest.Abi.GetMethod("main", 3);

            Assert.IsNotNull(method);
            var context = engine.LoadContract(contract, method!, CallFlags.All);
            var args = new StackItem[]
            {
                new Integer(128),
                new Integer(1),
                new Integer(381),
            };
            for (var i = args.Length - 1; i >= 0; i--)
                context.EvaluationStack.Push(args[i]);

            var state = engine.Execute();

            Assert.AreEqual(VMState.HALT, state);
            Assert.IsNotNull(bridge.LastRequest);
            Assert.AreEqual(3, bridge.LastRequest!.InitialStack.Count);
            Assert.AreEqual(381, bridge.LastRequest.InitialStack[0].GetInteger());
            Assert.AreEqual(1, bridge.LastRequest.InitialStack[1].GetInteger());
            Assert.AreEqual(128, bridge.LastRequest.InitialStack[2].GetInteger());
        }
        finally
        {
            ApplicationEngine.Provider = previousProvider;
        }
    }

    [TestMethod]
    public void UnloadNestedContextFromBridgeRestoresCurrentContextToCaller()
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
            new CapturingBridge());
        var contract = CreateNeoVmContractWithInitializer();
        var method = contract.Manifest.Abi.GetMethod("main", 0);

        Assert.IsNotNull(method);
        var methodContext = engine.LoadContract(contract, method!, CallFlags.All);
        var initContext = engine.CurrentContext;

        Assert.IsNotNull(initContext);
        Assert.AreNotSame(methodContext, initContext);

        engine.UnloadNestedContextFromBridge(
            initContext!,
            new RiscvExecutionResult(VMState.HALT, [], null));

        Assert.AreSame(methodContext, engine.CurrentContext);
    }

    [TestMethod]
    public void DiscardNestedContextFromBridgeRestoresCallerAfterInitializerContext()
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
            new CapturingBridge());
        var callerScript = new byte[] { (byte)OpCode.RET };
        var contract = CreateNeoVmContractWithInitializer();
        var method = contract.Manifest.Abi.GetMethod("main", 0);

        Assert.IsNotNull(method);
        engine.LoadScript(callerScript);
        var callerContext = engine.CurrentContext;
        var methodContext = engine.LoadContract(contract, method!, CallFlags.All);
        var initContext = engine.CurrentContext;
        methodContext.GetState<ExecutionContextState>().CallingContext = callerContext;

        Assert.IsNotNull(callerContext);
        Assert.IsNotNull(initContext);
        Assert.AreNotSame(methodContext, initContext);

        var discardedKey = new StorageKey
        {
            Id = contract.Id,
            Key = new byte[] { 0x7d },
        };
        var nestedSnapshot = methodContext.GetState<ExecutionContextState>().SnapshotCache;
        Assert.IsNotNull(nestedSnapshot);
        nestedSnapshot!.Add(discardedKey, new StorageItem(new byte[] { 0x01 }));

        engine.DiscardNestedContextFromBridge(methodContext);

        Assert.AreSame(callerContext, engine.CurrentContext);
        Assert.AreEqual(1, engine.InvocationStack.Count);
        Assert.IsNull(engine.SnapshotCache.TryGet(discardedKey));
    }

    [TestMethod]
    public void ExecutionFeeChargeReturnsFaultWhenGasIsExhausted()
    {
        var bridge = new CapturingBridge();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = new RiscvApplicationEngine(
            TriggerType.Application,
            null,
            snapshot,
            null,
            AdapterTestProtocolSettings.Default,
            gas: 0,
            bridge);

        var fault = NativeRiscvVmBridge.TryChargeExecutionFee(engine, feeConsumedPico: 1, "test");

        Assert.IsNotNull(fault);
        Assert.AreEqual(VMState.FAULT, fault!.State);
        Assert.AreEqual(0, fault.ResultStack.Count);
        Assert.IsInstanceOfType<InvalidOperationException>(fault.FaultException);
    }

    [TestMethod]
    public void NativeContractSnapshotIndexUsesPersistingBlockWhenAvailable()
    {
        var block = new Block
        {
            Header = new Header
            {
                Index = 42,
                PrevHash = UInt256.Zero,
                MerkleRoot = UInt256.Zero,
                NextConsensus = UInt160.Zero,
                Witness = Witness.Empty,
            },
            Transactions = [],
        };
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = new RiscvApplicationEngine(
            TriggerType.Application,
            null,
            snapshot,
            block,
            AdapterTestProtocolSettings.Default,
            ApplicationEngine.TestModeGas,
            new CapturingBridge());

        Assert.AreEqual(42u, NativeRiscvVmBridge.ResolveNativeContractSnapshotIndex(engine));
    }

    [TestMethod]
    public void NativeContractSnapshotIndexFallsBackToLedgerWhenNoPersistingBlockExists()
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
            new CapturingBridge());

        Assert.AreEqual(NativeContract.Ledger.CurrentIndex(snapshot), NativeRiscvVmBridge.ResolveNativeContractSnapshotIndex(engine));
    }

    private static ContractState CreateRiscVContract(string methodName = "main", int parameterCount = 0)
    {
        var script = new byte[] { 0x50, 0x56, 0x4d, 0x00, 0x01, 0x02, 0x03, 0x04 };
        var nef = new NefFile
        {
            Compiler = "test",
            Source = string.Empty,
            Tokens = [],
            Script = script,
        };
        nef.CheckSum = NefFile.ComputeChecksum(nef);

        return new ContractState
        {
            Id = 1,
            UpdateCounter = 0,
            Type = ContractType.RiscV,
            Hash = UInt160.Zero,
            Nef = nef,
            Manifest = new ContractManifest
            {
                Name = "DirectRiscV",
                Groups = [],
                SupportedStandards = [],
                Abi = new ContractAbi
                {
                    Methods =
                    [
                        new ContractMethodDescriptor
                        {
                            Name = methodName,
                            Parameters = Enumerable.Range(0, parameterCount)
                                .Select(index => new ContractParameterDefinition
                                {
                                    Name = $"arg{index}",
                                    Type = ContractParameterType.Integer,
                                })
                                .ToArray(),
                            ReturnType = ContractParameterType.Void,
                            Offset = 0,
                            Safe = false,
                        }
                    ],
                    Events = [],
                },
                Permissions = [ContractPermission.DefaultPermission],
                Trusts = WildcardContainer<ContractPermissionDescriptor>.CreateWildcard(),
            },
        };
    }

    private static ContractState CreateNeoVmContractWithInitializer()
    {
        var script = new byte[]
        {
            (byte)OpCode.RET,
            (byte)OpCode.RET,
        };
        var nef = new NefFile
        {
            Compiler = "test",
            Source = string.Empty,
            Tokens = [],
            Script = script,
        };
        nef.CheckSum = NefFile.ComputeChecksum(nef);

        return new ContractState
        {
            Id = 2,
            UpdateCounter = 0,
            Type = ContractType.NeoVM,
            Hash = script.ToScriptHash(),
            Nef = nef,
            Manifest = new ContractManifest
            {
                Name = "NeoVmWithInitializer",
                Groups = [],
                SupportedStandards = [],
                Abi = new ContractAbi
                {
                    Methods =
                    [
                        new ContractMethodDescriptor
                        {
                            Name = "main",
                            Parameters = [],
                            ReturnType = ContractParameterType.Void,
                            Offset = 0,
                            Safe = false,
                        },
                        new ContractMethodDescriptor
                        {
                            Name = ContractBasicMethod.Initialize,
                            Parameters = [],
                            ReturnType = ContractParameterType.Void,
                            Offset = 1,
                            Safe = false,
                        },
                    ],
                    Events = [],
                },
                Permissions = [ContractPermission.DefaultPermission],
                Trusts = WildcardContainer<ContractPermissionDescriptor>.CreateWildcard(),
            },
        };
    }

    private sealed class CapturingBridge : IRiscvVmBridge
    {
        public RiscvExecutionRequest? LastRequest { get; private set; }

        public RiscvExecutionResult Execute(RiscvExecutionRequest request)
        {
            LastRequest = request;
            return new RiscvExecutionResult(VMState.HALT, [], null);
        }

        public RiscvExecutionResult ExecuteContract(
            ApplicationEngine engine,
            ContractState contract,
            string method,
            CallFlags flags,
            IReadOnlyList<StackItem> args)
        {
            return Execute(new RiscvExecutionRequest(
                engine,
                engine.Trigger,
                engine.ProtocolSettings.Network,
                engine.ProtocolSettings.AddressVersion,
                engine.PersistingBlock?.Timestamp ?? 0,
                engine.GasLeft,
                flags,
                [contract.Script.ToArray()],
                [contract.Hash],
                [contract.Type],
                [contract.Hash],
                args,
                method: method));
        }
    }
}
