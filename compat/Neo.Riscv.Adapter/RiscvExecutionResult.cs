// Copyright (C) 2015-2026 The Neo Project.
//
// RiscvExecutionResult.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.VM;
using Neo.VM.Types;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Neo.SmartContract.RiscV
{
    public sealed class RiscvExecutionResult
    {
        public VMState State { get; }

        public IReadOnlyList<StackItem> ResultStack { get; }

        public Exception? FaultException { get; }

        /// <summary>
        /// Instruction pointer (NEF script offset) at which the FAULT was raised, or
        /// <see langword="null"/> for HALT / when the native side did not attribute an IP
        /// (sentinel value <c>uint.MaxValue</c>). Used by <see cref="RiscvApplicationEngine"/>
        /// to restore the faulting <see cref="ExecutionContext.InstructionPointer"/> so
        /// dev-time tests asserting exact fault offsets see the real opcode offset.
        /// </summary>
        public int? FaultIp { get; }

        public RiscvExecutionResult(VMState state, IEnumerable<StackItem> resultStack, Exception? faultException, int? faultIp = null)
        {
            State = state;
            ResultStack = resultStack?.ToArray() ?? throw new ArgumentNullException(nameof(resultStack));
            FaultException = faultException;
            FaultIp = faultIp;
        }
    }
}
