// Copyright (C) 2015-2026 The Neo Project.
//
// IRiscvApplicationEngineTestingHooks.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.Persistence;
using Neo.SmartContract.Manifest;
using Neo.VM.Types;

namespace Neo.SmartContract.RiscV
{
    /// <summary>
    /// Optional extension point implemented by consumers (notably the devpack
    /// testing framework in <c>Neo.SmartContract.Testing</c>) to intercept
    /// adapter execution for mocking, override semantics, and coverage tracking.
    /// The adapter invokes each hook only when <see cref="RiscvApplicationEngine.TestingHooks"/>
    /// is set — production nodes leave it <see langword="null"/> and incur no overhead.
    /// </summary>
    public interface IRiscvApplicationEngineTestingHooks
    {
        UInt160? OverrideCallingScriptHash(UInt160? current, UInt160? expected);

        UInt160? OverrideEntryScriptHash(UInt160? current, UInt160? expected);

        bool TryInvokeCustomMock(ApplicationEngine engine, DataCache snapshot, UInt160 contractHash, string method, StackItem[] args, out StackItem result);

        void RecordMethodCoverage(UInt160 contractHash, ContractState contractState, ContractMethodDescriptor descriptor);
    }
}
