using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.SmartContract.RiscV;
using System;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public class UT_RiscvContractCallCompatibility
{
    [TestMethod]
    public void ContractCallRejectsPrivateMethodNames()
    {
        Assert.ThrowsExactly<ArgumentException>(() =>
            NativeRiscvVmBridge.ValidateContractCallMethodForTesting("_initialize", isCallT: false));
    }

    [TestMethod]
    public void CallTAllowsTokenMethodNamesWithLeadingUnderscore()
    {
        NativeRiscvVmBridge.ValidateContractCallMethodForTesting("_initialize", isCallT: true);
    }
}
