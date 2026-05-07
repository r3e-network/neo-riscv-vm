using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.Collections.Generic;
using System.Numerics;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public class UT_NeoVmReflectionAccess
{
    [TestMethod]
    public void RequiredNeoVmPrivateAccessorsAreAvailable()
    {
        Assert.IsTrue(NeoVmExecutionContextAccessors.StrictModeAccessorAvailableForTesting);
        Assert.IsTrue(NeoVmExecutionContextAccessors.CurrentContextAccessorAvailableForTesting);
        Assert.IsTrue(NeoVmExecutionContextAccessors.FaultInstructionPointerAccessorAvailableForTesting);
        Assert.IsTrue(NeoVmExecutionContextAccessors.FaultLocalVariablesAccessorAvailableForTesting);
    }

    [TestMethod]
    public void StrictModeAccessorReadsNeoVmScriptFlag()
    {
        Assert.IsTrue(NeoVmExecutionContextAccessors.IsStrictMode(new Script(new byte[] { (byte)OpCode.RET }, strictMode: true)));
        Assert.IsFalse(NeoVmExecutionContextAccessors.IsStrictMode(new Script(new byte[] { (byte)OpCode.RET }, strictMode: false)));
    }

    [TestMethod]
    public void FaultMetadataAccessorsMutateCurrentContext()
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

        engine.LoadScript(new byte[] { (byte)OpCode.RET });
        var context = engine.CurrentContext!;

        Assert.IsTrue(NeoVmExecutionContextAccessors.TrySetInstructionPointer(context, 7));
        Assert.AreEqual(7, context.InstructionPointer);

        Assert.IsTrue(NeoVmExecutionContextAccessors.TrySetLocalVariables(
            context,
            new Slot([new Integer(new BigInteger(42))], engine.ReferenceCounter)));
        Assert.AreEqual(new BigInteger(42), context.LocalVariables![0].GetInteger());
    }

    [TestMethod]
    public void CurrentContextAccessorMutatesExecutionEngineState()
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

        engine.LoadScript(new byte[] { (byte)OpCode.RET });
        Assert.IsNotNull(engine.CurrentContext);

        NeoVmExecutionContextAccessors.SetCurrentContext(engine, null);
        Assert.IsNull(engine.CurrentContext);
    }

    private sealed class NoopBridge : IRiscvVmBridge
    {
        public RiscvExecutionResult Execute(RiscvExecutionRequest request)
        {
            throw new NotSupportedException();
        }

        public RiscvExecutionResult ExecuteContract(
            ApplicationEngine engine,
            ContractState contract,
            string method,
            CallFlags flags,
            IReadOnlyList<StackItem> args)
        {
            throw new NotSupportedException();
        }
    }
}
