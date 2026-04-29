using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Network.P2P.Payloads;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.RiscV;
using Neo.VM;
using Neo.VM.Types;
using System.Collections.Generic;

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
