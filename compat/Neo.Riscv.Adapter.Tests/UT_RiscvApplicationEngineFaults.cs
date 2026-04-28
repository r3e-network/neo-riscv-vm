using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.Collections.Generic;
using System.Reflection;
using System.Text;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
[DoNotParallelize]
public class UT_RiscvApplicationEngineFaults
{
    [TestMethod]
    public void FaultedExecutionRollsBackCurrentContextNotifications()
    {
        var script = new byte[] { (byte)OpCode.RET };
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var engine = new RiscvApplicationEngine(
            TriggerType.Application,
            null,
            snapshot,
            null,
            AdapterTestProtocolSettings.Default,
            ApplicationEngine.TestModeGas,
            new NotifyingFaultBridge());

        engine.LoadScript(script);
        engine.CurrentContext!.GetState<ExecutionContextState>().Contract = CreateContract(script);

        Assert.AreEqual(VMState.FAULT, engine.Execute());
        Assert.AreEqual(0, engine.Notifications.Count);
    }

    private static ContractState CreateContract(byte[] script)
    {
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
            Type = ContractType.NeoVM,
            Hash = script.ToScriptHash(),
            Nef = nef,
            Manifest = new ContractManifest
            {
                Name = "NotifyingFault",
                Groups = [],
                SupportedStandards = [],
                Abi = new ContractAbi
                {
                    Methods = [],
                    Events =
                    [
                        new ContractEventDescriptor
                        {
                            Name = "evt",
                            Parameters = [],
                        }
                    ],
                },
                Permissions = [],
                Trusts = WildcardContainer<ContractPermissionDescriptor>.CreateWildcard(),
            },
        };
    }

    private sealed class NotifyingFaultBridge : IRiscvVmBridge
    {
        private static readonly MethodInfo RuntimeNotifyMethod =
            typeof(ApplicationEngine).GetMethod("RuntimeNotify", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? throw new MissingMethodException(nameof(ApplicationEngine), "RuntimeNotify");

        public RiscvExecutionResult Execute(RiscvExecutionRequest request)
        {
            RuntimeNotifyMethod.Invoke(
                request.Engine,
                [Encoding.UTF8.GetBytes("evt"), new Neo.VM.Types.Array(request.Engine.ReferenceCounter)]);
            return new RiscvExecutionResult(VMState.FAULT, [], new InvalidOperationException("fault"));
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
