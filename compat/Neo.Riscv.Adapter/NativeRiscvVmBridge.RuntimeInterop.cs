using Neo.VM;
using Neo.VM.Types;
using Neo.SmartContract.Native;
using System;
using System.Linq;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private static StackItem[] HandleRuntimeLog(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Runtime.Log requires one argument.");

            if (inputStack[^1] is not ByteString message)
                throw new InvalidOperationException("Runtime.Log requires a byte string argument.");

            request.Engine.RuntimeLog(message.GetSpan().ToArray());

            var next = new StackItem[inputStack.Length - 1];
            System.Array.Copy(inputStack, next, next.Length);
            return next;
        }

        private static StackItem[] HandleRuntimeNotify(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("Runtime.Notify requires event name and state.");

            if (inputStack[^1] is not ByteString eventName)
                throw new InvalidOperationException("Runtime.Notify requires a byte string event name.");
            if (inputStack[^2] is not Neo.VM.Types.Array state)
                throw new InvalidOperationException("Runtime.Notify requires an array state.");

            request.Engine.RuntimeNotify(eventName.GetSpan().ToArray(), state);

            var next = new StackItem[inputStack.Length - 2];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private static StackItem[] HandleBurnGas(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Runtime.BurnGas requires one argument.");

            if (inputStack[^1] is not Integer amount)
                throw new InvalidOperationException("Runtime.BurnGas requires an integer argument.");

            request.Engine.BurnGas((long)amount.GetInteger());

            var next = new StackItem[inputStack.Length - 1];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private StackItem[] HandleGetInvocationCounter(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            var count = request.ScriptHashes.Count(scriptHash => scriptHash.Equals(request.ScriptHashes[^1]));
            var contract = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, request.ScriptHashes[^1]);
            if (contract is not null)
            {
                var engineCount = request.Engine.ExecuteInNativeContractContext(
                    request.ScriptHashes[^1],
                    request.ScriptHashes.Count > 1 ? request.ScriptHashes[^2] : null,
                    contract,
                    request.CurrentCallFlags,
                    () => request.Engine.GetInvocationCounter());
                count = Math.Max(count, engineCount);
            }
            return Append(inputStack, new Integer(count));
        }

        private StackItem[] HandleRuntimeLoadScript(RiscvExecutionRequest request, ExecutionScope scope, long gasLeft, StackItem[] inputStack)
        {
            if (inputStack.Length < 3)
                throw new InvalidOperationException("Runtime.LoadScript requires script, call flags, and args.");

            if (inputStack[^1] is not ByteString scriptItem)
                throw new InvalidOperationException("Runtime.LoadScript requires a byte string script.");
            if (inputStack[^2] is not Integer callFlagsItem)
                throw new InvalidOperationException("Runtime.LoadScript requires integer call flags.");
            if (inputStack[^3] is not Neo.VM.Types.Array argsArray)
                throw new InvalidOperationException("Runtime.LoadScript requires an array of arguments.");

            var callFlags = (CallFlags)(byte)callFlagsItem.GetInteger();
            if ((callFlags & ~CallFlags.All) != 0)
                throw new InvalidOperationException($"Invalid call flags: {callFlags}");

            var nestedScript = scriptItem.GetSpan().ToArray();
            if (IsPolkaVmBinary(nestedScript))
                throw new InvalidOperationException("Runtime.LoadScript does not support direct PolkaVM binaries.");

            var nextScripts = request.Scripts.Concat(new[] { nestedScript }).ToArray();
            var nextHashes = request.ScriptHashes.Concat(new[] { nestedScript.ToScriptHash() }).ToArray();
            var nextContractTypes = request.ContractTypes.Concat(new[] { ContractType.NeoVM }).ToArray();
            var nextExecutionFacadeHashes = nextContractTypes
                .Zip(nextHashes, (contractType, scriptHash) =>
                    RiscvCompatibilityContracts.ResolveExecutionFacadeHash(contractType, scriptHash))
                .ToArray();
            var nestedCallFlags = callFlags & request.CurrentCallFlags & CallFlags.ReadOnly;
            var nestedRequest = new RiscvExecutionRequest(
                request.Engine,
                request.Trigger,
                request.NetworkMagic,
                request.AddressVersion,
                request.PersistingTimestamp,
                gasLeft,
                nestedCallFlags,
                nextScripts,
                nextHashes,
                nextContractTypes,
                nextExecutionFacadeHashes);

            var nestedInitialStack = new StackItem[argsArray.Count];
            for (var index = 0; index < argsArray.Count; index++)
            {
                nestedInitialStack[index] = argsArray[argsArray.Count - index - 1];
            }

            var callerContext = request.Engine.CurrentContext!;
            var callerState = callerContext.GetState<ExecutionContextState>();
            var nestedContext = request.Engine.LoadScript(
                new Script(nestedScript, true),
                configureState: state =>
                {
                    state.CallingContext = callerContext;
                    state.CallFlags = nestedCallFlags;
                    state.IsDynamicCall = true;
                });
            var nestedState = nestedContext.GetState<ExecutionContextState>();
            var nestedSnapshot = nestedState.SnapshotCache;

            for (var index = nestedInitialStack.Length - 1; index >= 0; index--)
                nestedContext.EvaluationStack.Push(nestedInitialStack[index]);

            RiscvExecutionResult nestedResult;
            nestedResult = request.Engine.ExecuteInNativeContractContext(
                nestedScript.ToScriptHash(),
                request.ScriptHashes[^1],
                null,
                nestedCallFlags,
                () => ExecuteScriptInternal(nestedRequest, nestedScript, nestedInitialStack, 0, scope));

            if (nestedResult.State == VMState.HALT)
            {
                if (request.Engine is RiscvApplicationEngine riscvEngine)
                    riscvEngine.UnloadNestedContextFromBridge(nestedContext, nestedResult);
                else
                {
                    nestedSnapshot?.Commit();
                    callerState.NotificationCount += nestedState.NotificationCount;
                    PopNestedContextIfCurrent(request.Engine, nestedContext);
                }
            }
            else
            {
                scope.PendingNestedFault = nestedResult;
                throw nestedResult.FaultException ?? new InvalidOperationException("Runtime.LoadScript failed.");
            }

            return BuildDynamicCallReturnStack(inputStack, 3, nestedResult.ResultStack);
        }

        private static bool IsPolkaVmBinary(byte[] script)
        {
            return script.Length >= 4
                && script[0] == 0x50
                && script[1] == 0x56
                && script[2] == 0x4D
                && script[3] == 0x00;
        }

        private static StackItem[] HandleGetNotifications(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Runtime.GetNotifications requires one argument.");

            UInt160? hash = inputStack[^1] switch
            {
                Null => null,
                ByteString bytes when bytes.GetSpan().Length == UInt160.Length => new UInt160(bytes.GetSpan()),
                _ => throw new InvalidOperationException("Runtime.GetNotifications requires null or a UInt160 byte string.")
            };

            var notifications = request.Engine.GetNotifications(hash);
            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = notifications;
            return next;
        }

        private static StackItem[] HandleCheckWitness(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Runtime.CheckWitness requires one argument.");

            if (inputStack[^1] is Null)
                throw new ArgumentException("The argument `hashOrPubkey` can't be null.");
            if (!TryGetByteLikeBytes(inputStack[^1], out var data))
                throw new InvalidOperationException("Runtime.CheckWitness requires a byte string argument.");

            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }

            next[^1] = request.Engine.CheckWitness(data) ? StackItem.True : StackItem.False;
            return next;
        }

        private static StackItem CreateScriptContainerItem(RiscvExecutionRequest request)
        {
            if (request.Engine.ScriptContainer is null)
                return request.Engine.GetScriptContainer();

            return request.Engine.Convert(request.Engine.ScriptContainer);
        }
    }
}
