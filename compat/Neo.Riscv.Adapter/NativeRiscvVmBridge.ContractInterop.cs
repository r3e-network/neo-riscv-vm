using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.Native;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.Linq;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private StackItem[] HandleContractCallViaRiscv(
            RiscvExecutionRequest request,
            ExecutionScope scope,
            long gasLeft,
            StackItem[] inputStack,
            ContractState contract,
            ContractMethodDescriptor descriptor,
            CallFlags callFlags,
            Neo.VM.Types.Array argsArray)
        {
            request.Engine.IncrementInvocationCounter(contract.Hash);

            if (NativeContract.Policy.IsBlocked(request.Engine.SnapshotCache, contract.Hash))
                throw new InvalidOperationException($"The contract {contract.Hash} has been blocked.");

            if (!descriptor.Safe)
            {
                var currentState = request.Engine.CurrentContext!.GetState<ExecutionContextState>();
                var executingContract = request.Engine.IsHardforkEnabled(Hardfork.HF_Domovoi)
                    ? currentState.Contract
                    : NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, request.Engine.CurrentScriptHash!);
                if (executingContract?.CanCall(contract, descriptor.Name) == false)
                    throw new InvalidOperationException($"Cannot Call Method {descriptor.Name} Of Contract {contract.Hash} From Contract {request.Engine.CurrentScriptHash}");
            }

            var calleeScript = contract.Script.ToArray();
            // If the contract has an `_initialize` method (populates static fields), classic
            // NeoVM's LoadContract pushes the _initialize context on top of the method's
            // context so statics are set before the user method runs. The adapter dispatches
            // the method directly at descriptor.Offset and doesn't push per-call contexts, so
            // we emulate the effect by prepending a wrapper — `CALL_L _init; JMP_L method;
            // [original script]` — and starting execution at the wrapper's offset 0. Statics
            // set by _initialize persist through the trailing JMP into the main method body.
            var executionScript = BuildInitializeWrapperIfNeeded(contract, descriptor, calleeScript);
            var nestedScripts = request.Scripts.Concat(new[] { calleeScript }).ToArray();
            var nestedContractTypes = request.ContractTypes.Concat(new[] { contract.Type }).ToArray();
            var nestedScriptHashes = request.ScriptHashes.Concat(new[] { contract.Hash }).ToArray();
            var nestedExecutionFacadeHashes = request.ExecutionFacadeHashes
                .Concat(new[] { RiscvCompatibilityContracts.ResolveExecutionFacadeHash(contract.Type, contract.Hash) })
                .ToArray();
            var nestedRequest = new RiscvExecutionRequest(
                request.Engine,
                request.Trigger,
                request.NetworkMagic,
                request.AddressVersion,
                request.PersistingTimestamp,
                gasLeft,
                ((descriptor.Safe ? callFlags & ~(CallFlags.WriteStates | CallFlags.AllowNotify) : callFlags) & request.CurrentCallFlags),
                nestedScripts,
                nestedScriptHashes,
                nestedContractTypes,
                nestedExecutionFacadeHashes);

            var nestedInitialStack = new StackItem[argsArray.Count];
            for (var index = 0; index < argsArray.Count; index++)
            {
                nestedInitialStack[index] = argsArray[argsArray.Count - index - 1];
            }

            // Mirror classic NeoVM LoadContract (ApplicationEngine.cs:422-429): charge fixedFee
            // and flag the current context so subsequent AddFee calls inside the callee are
            // suppressed. Restore the flag after the nested call to avoid leaking scope.
            ExecutionContextState? whitelistState = null;
            var previousWhiteListed = false;
            if (request.Engine.IsHardforkEnabled(Hardfork.HF_Faun) &&
                NativeContract.Policy.IsWhitelistFeeContract(request.Engine.SnapshotCache, contract.Hash, descriptor, out var fixedFee))
            {
                request.Engine.AddFee(fixedFee!.Value * ApplicationEngine.FeeFactor);
                whitelistState = request.Engine.CurrentContext?.GetState<ExecutionContextState>();
                if (whitelistState is not null)
                {
                    previousWhiteListed = whitelistState.WhiteListed;
                    whitelistState.WhiteListed = true;
                }
            }

            // Clone the snapshot for the nested call so storage writes from a FAULTing callee
            // do not leak into the caller's snapshot view. Mirrors classic NeoVM where
            // LoadScript creates a new ExecutionContext with its own cloned SnapshotCache
            // (ApplicationEngine.cs:774) and ContextUnloaded only commits on HALT (:597).
            // On HALT we commit the clone so the caller sees the writes; on FAULT we drop
            // the clone silently and the caller's snapshot is untouched. Restoration happens
            // in the finally block regardless.
            var contextState = request.Engine.CurrentContext?.GetState<ExecutionContextState>();
            var previousSnapshot = contextState?.SnapshotCache;
            var nestedSnapshot = previousSnapshot?.CloneCache();
            if (contextState is not null) contextState.SnapshotCache = nestedSnapshot;

            RiscvExecutionResult nestedResult;
            try
            {
                nestedResult = request.Engine.ExecuteInNativeContractContext(
                    contract.Hash,
                    request.ScriptHashes[^1],
                    contract,
                    nestedRequest.CurrentCallFlags,
                    () => contract.Type == ContractType.RiscV
                        ? ExecuteNativeContractInternal(nestedRequest, calleeScript, nestedInitialStack, descriptor.Name, scope)
                        : ExecuteScriptInternal(
                            nestedRequest,
                            executionScript ?? calleeScript,
                            nestedInitialStack,
                            executionScript is null ? descriptor.Offset : 0,
                            scope));
            }
            finally
            {
                if (contextState is not null) contextState.SnapshotCache = previousSnapshot;
                if (whitelistState is not null) whitelistState.WhiteListed = previousWhiteListed;
            }
            if (nestedResult.State == VMState.HALT)
            {
                nestedSnapshot?.Commit();
            }
            else
            {
                throw nestedResult.FaultException ?? new InvalidOperationException("Contract.Call failed.");
            }

            if (request.Engine is RiscvApplicationEngine riscvEngine)
                riscvEngine.TestingHooks?.RecordMethodCoverage(contract.Hash, contract, descriptor);

            Trace($"contract.call nested exit method={descriptor.Name} resultCount={nestedResult.ResultStack.Count}");

            var returnedCount = nestedResult.ResultStack.Count == 0 ? 1 : nestedResult.ResultStack.Count;
            var next = new StackItem[inputStack.Length - 4 + returnedCount];
            if (inputStack.Length > 4)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 4);
            }
            if (nestedResult.ResultStack.Count == 0)
            {
                next[^1] = StackItem.Null;
            }
            for (var index = 0; index < nestedResult.ResultStack.Count; index++)
            {
                next[inputStack.Length - 4 + index] = nestedResult.ResultStack[index];
            }

            return next;
        }

        /// <summary>
        /// Builds a wrapper script that calls <c>_initialize</c> (to populate static fields)
        /// before jumping into the user method. Returns <see langword="null"/> if the contract
        /// has no <c>_initialize</c> method, in which case the caller executes the original
        /// script at <c>descriptor.Offset</c>. The wrapper prepends 10 bytes:
        /// <c>CALL_L (5 bytes) + JMP_L (5 bytes)</c>, so all original offsets shift by 10 in
        /// the wrapper coordinate space; both relative jumps account for that.
        /// </summary>
        private static byte[]? BuildInitializeWrapperIfNeeded(
            ContractState contract,
            ContractMethodDescriptor descriptor,
            byte[] originalScript)
        {
            var initMethod = contract.Manifest.Abi.GetMethod(
                ContractBasicMethod.Initialize,
                ContractBasicMethod.InitializePCount);
            if (initMethod is null) return null;

            // CALL_L (0x35) — 32-bit signed offset relative to the CALL_L instruction itself.
            // JMP_L  (0x23) — same semantics, relative to the JMP_L instruction.
            // Wrapper layout (offsets in the wrapper coordinate space):
            //   0..4  : CALL_L  imm = 10 + initMethod.Offset   (target = _initialize in the
            //                                                    wrapped original script)
            //   5..9  : JMP_L   imm = 5 + descriptor.Offset    (target = user method)
            //   10... : original script verbatim
            const int wrapperPrefixLen = 10;
            var wrapper = new byte[wrapperPrefixLen + originalScript.Length];

            wrapper[0] = (byte)OpCode.CALL_L;
            WriteInt32Le(wrapper, 1, wrapperPrefixLen + initMethod.Offset);
            wrapper[5] = (byte)OpCode.JMP_L;
            WriteInt32Le(wrapper, 6, (wrapperPrefixLen - 5) + descriptor.Offset);
            System.Array.Copy(originalScript, 0, wrapper, wrapperPrefixLen, originalScript.Length);

            return wrapper;
        }

        private static void WriteInt32Le(byte[] buffer, int offset, int value)
        {
            buffer[offset] = (byte)value;
            buffer[offset + 1] = (byte)(value >> 8);
            buffer[offset + 2] = (byte)(value >> 16);
            buffer[offset + 3] = (byte)(value >> 24);
        }
    }
}
