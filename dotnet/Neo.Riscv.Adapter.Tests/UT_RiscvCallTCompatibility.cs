using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.RiscV;
using System;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public class UT_RiscvCallTCompatibility
{
    [TestMethod]
    public void CallTRequiresReadStatesAndAllowCall()
    {
        Assert.ThrowsExactly<InvalidOperationException>(() =>
            NativeRiscvVmBridge.ValidateCallTForTesting(
                CallFlags.ReadStates,
                tokenHasReturnValue: true,
                ContractParameterType.Integer));
    }

    [TestMethod]
    public void CallTRejectsTokenReturnShapeMismatch()
    {
        Assert.ThrowsExactly<InvalidOperationException>(() =>
            NativeRiscvVmBridge.ValidateCallTForTesting(
                CallFlags.ReadStates | CallFlags.AllowCall,
                tokenHasReturnValue: true,
                ContractParameterType.Void));
    }

    [TestMethod]
    public void CallTAcceptsVoidTokenForVoidMethod()
    {
        NativeRiscvVmBridge.ValidateCallTForTesting(
            CallFlags.All,
            tokenHasReturnValue: false,
            ContractParameterType.Void);
    }

    [TestMethod]
    public void CallTAcceptsReturnTokenForReturningMethod()
    {
        NativeRiscvVmBridge.ValidateCallTForTesting(
            CallFlags.ReadStates | CallFlags.AllowCall,
            tokenHasReturnValue: true,
            ContractParameterType.Integer);
    }
}
