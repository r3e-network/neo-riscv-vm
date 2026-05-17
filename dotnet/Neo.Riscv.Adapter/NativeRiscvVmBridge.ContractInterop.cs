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
            Neo.VM.Types.Array argsArray,
            bool isDynamicCall)
        {
            if (NativeContract.Policy.IsBlocked(request.Engine.SnapshotCache, contract.Hash))
                throw new InvalidOperationException($"The contract {contract.Hash} has been blocked.");

            var callerContext = request.Engine.CurrentContext!;
            var callerState = callerContext.GetState<ExecutionContextState>();
            if (!descriptor.Safe)
            {
                var executingContract = request.Engine.IsHardforkEnabled(Hardfork.HF_Domovoi)
                    ? callerState.Contract
                    : NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, request.ScriptHashes[^1]);
                if (executingContract?.CanCall(contract, descriptor.Name) == false)
                    throw new InvalidOperationException($"Cannot Call Method {descriptor.Name} Of Contract {contract.Hash} From Contract {request.ScriptHashes[^1]}");
            }

            request.Engine.IncrementInvocationCounter(contract.Hash);

            var calleeScript = contract.Script.ToArray();
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

            var executionContractState = new ContractState
            {
                Id = contract.Id,
                UpdateCounter = contract.UpdateCounter,
                Type = contract.Type,
                Hash = contract.Hash,
                Nef = contract.Nef,
                Manifest = contract.Manifest
            };
            var nestedContext = request.Engine.LoadContract(contract, descriptor, nestedRequest.CurrentCallFlags);
            var nestedState = nestedContext.GetState<ExecutionContextState>();
            nestedState.CallingContext = callerContext;
            nestedState.IsDynamicCall = isDynamicCall;
            var nestedSnapshot = nestedState.SnapshotCache;

            // Mirror classic NeoVM LoadContract (ApplicationEngine.cs:422-429): charge fixedFee
            // and flag the callee context so subsequent AddFee calls inside the callee are
            // suppressed.
            if (request.Engine.IsHardforkEnabled(Hardfork.HF_Faun) &&
                NativeContract.Policy.IsWhitelistFeeContract(request.Engine.SnapshotCache, contract.Hash, descriptor, out var fixedFee))
            {
                request.Engine.AddFee(fixedFee!.Value * ApplicationEngine.FeeFactor);
                nestedState.WhiteListed = true;
            }

            for (var index = nestedInitialStack.Length - 1; index >= 0; index--)
                nestedContext.EvaluationStack.Push(nestedInitialStack[index]);

            var initMethod = contract.Type == ContractType.RiscV
                ? null
                : contract.Manifest.Abi.GetMethod(
                    ContractBasicMethod.Initialize,
                    ContractBasicMethod.InitializePCount);
            var initializerInstructionPointer = (int?)null;
            Neo.VM.ExecutionContext? initContext = null;
            if (initMethod is not null && request.Engine.CurrentContext is { } currentInitContext && !ReferenceEquals(currentInitContext, nestedContext))
            {
                initContext = currentInitContext;
                initializerInstructionPointer = initMethod.Offset;
                scope.PendingInitializerContexts.Push(initContext);
            }
            DiagnosticTrace(request, "contract.call.nested",
                $"target={contract.Hash} type={contract.Type} method={descriptor.Name} returnType={descriptor.ReturnType} initIp={initializerInstructionPointer?.ToString() ?? "<none>"}");

            RiscvExecutionResult nestedResult;
            try
            {
                nestedResult = request.Engine.ExecuteInNativeContractContext(
                    contract.Hash,
                    request.ScriptHashes[^1],
                    executionContractState,
                    nestedRequest.CurrentCallFlags,
                    () => contract.Type == ContractType.RiscV
                        ? ExecuteNativeContractInternal(nestedRequest, calleeScript, nestedInitialStack, descriptor.Name, scope)
                        : ExecuteScriptInternal(
                            nestedRequest,
                            calleeScript,
                            nestedInitialStack,
                            descriptor.Offset,
                            scope,
                            initializerInstructionPointer: initializerInstructionPointer,
                            resultStackLimit: descriptor.ReturnType == ContractParameterType.Void ? 0 : 1));
            }
            finally
            {
                if (initContext is not null &&
                    scope.PendingInitializerContexts.Count > 0 &&
                    ReferenceEquals(scope.PendingInitializerContexts.Peek(), initContext))
                {
                    scope.PendingInitializerContexts.Pop();
                }
            }

            if (nestedResult.State == VMState.HALT)
            {
                if (request.Engine is RiscvApplicationEngine nestedEngine)
                    nestedEngine.UnloadNestedContextFromBridge(nestedContext, nestedResult);
                else
                {
                    nestedSnapshot?.Commit();
                    callerState.NotificationCount += nestedState.NotificationCount;
                    PopNestedContextIfCurrent(request.Engine, nestedContext);
                }
            }
            else
            {
                if (request.Engine is RiscvApplicationEngine nestedEngine)
                    nestedEngine.DiscardNestedContextFromBridge(nestedContext);
                else
                    PopNestedContextIfCurrent(request.Engine, nestedContext);

                scope.PendingNestedFault = nestedResult;
                throw nestedResult.FaultException ?? new InvalidOperationException("Contract.Call failed.");
            }

            if (request.Engine is RiscvApplicationEngine riscvEngine)
                riscvEngine.TestingHooks?.RecordMethodCoverage(contract.Hash, contract, descriptor);

            Trace($"contract.call nested exit method={descriptor.Name} resultCount={nestedResult.ResultStack.Count}");

            return isDynamicCall
                ? BuildDynamicContractCallReturnStack(inputStack, 4, nestedResult.ResultStack)
                : BuildContractCallReturnStack(inputStack, 4, descriptor.ReturnType, nestedResult.ResultStack);
        }

        internal static StackItem[] BuildContractCallReturnStack(
            StackItem[] inputStack,
            int consumedArgumentCount,
            ContractParameterType returnType,
            System.Collections.Generic.IReadOnlyList<StackItem> resultStack)
        {
            if (inputStack is null) throw new ArgumentNullException(nameof(inputStack));
            if (resultStack is null) throw new ArgumentNullException(nameof(resultStack));
            if (consumedArgumentCount < 0 || inputStack.Length < consumedArgumentCount)
                throw new ArgumentOutOfRangeException(nameof(consumedArgumentCount));

            var returnedCount = returnType == ContractParameterType.Void ? 0 : 1;
            if (returnType != ContractParameterType.Void && resultStack.Count == 0)
                throw new InvalidOperationException("Contract.Call target did not return a value for a non-void method.");

            var prefixLength = inputStack.Length - consumedArgumentCount;
            var next = new StackItem[prefixLength + returnedCount];
            if (prefixLength > 0)
            {
                System.Array.Copy(inputStack, next, prefixLength);
            }
            if (returnedCount == 1)
                next[prefixLength] = resultStack[^1];

            return next;
        }

        internal static StackItem[] BuildDynamicContractCallReturnStack(
            StackItem[] inputStack,
            int consumedArgumentCount,
            System.Collections.Generic.IReadOnlyList<StackItem> resultStack)
        {
            return BuildDynamicCallReturnStack(inputStack, consumedArgumentCount, resultStack);
        }

        internal static StackItem[] BuildDynamicCallReturnStack(
            StackItem[] inputStack,
            int consumedArgumentCount,
            System.Collections.Generic.IReadOnlyList<StackItem> resultStack)
        {
            if (inputStack is null) throw new ArgumentNullException(nameof(inputStack));
            if (resultStack is null) throw new ArgumentNullException(nameof(resultStack));
            if (consumedArgumentCount < 0 || inputStack.Length < consumedArgumentCount)
                throw new ArgumentOutOfRangeException(nameof(consumedArgumentCount));
            if (resultStack.Count > 1)
                throw new NotSupportedException("Multiple return values are not allowed in cross-contract calls.");

            var prefixLength = inputStack.Length - consumedArgumentCount;
            var next = new StackItem[prefixLength + 1];
            if (prefixLength > 0)
            {
                System.Array.Copy(inputStack, next, prefixLength);
            }
            next[prefixLength] = resultStack.Count == 0 ? StackItem.Null : resultStack[0];

            return next;
        }

    }
}
